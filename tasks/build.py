"""
Build tasks.
"""
from __future__ import print_function

from itertools import chain, imap
import logging
import os
from pathlib import Path
import re
try:
    from shlex import quote
except ImportError:
    from pipes import quote
import shutil
import sys

from invoke import task
import semver

from tasks import BIN, LIB
from tasks.util import cargo, docs


MIN_RUSTC_VERSION = '1.10.0'

HELP = {
    'release': "Whether to build artifacts in release mode",
    'verbose': "Whether to show verbose logging output of the build",
}


@task(help=HELP, default=True)
def all(ctx, release=False, verbose=False):
    """Build the project."""
    # calling lib() is unnecessary because the binary crate
    # depeends on the library crate, so it will be rebuilt as well
    bin(ctx, release=release, verbose=verbose)
    docs_(ctx, release=release, verbose=verbose)
    print("\nBuild finished.", file=sys.stderr)


@task(help=HELP)
def bin(ctx, release=False, verbose=False):
    """Build the binary crate."""
    ensure_rustc_version(ctx)
    cargo_build = cargo(ctx, 'build', *get_rustc_flags(release, verbose),
                        crate=BIN, pty=True)
    if not cargo_build.ok:
        logging.critical("Failed to build the binary crate")
        return cargo_build.return_code

    # TODO: everything below should probably its own separate task

    # run the resulting binary to obtain command line help
    binary = cargo(ctx, 'run', crate=BIN, hide=True)
    if not binary.ok:
        logging.critical("Compiled binary return error code %s; stderr:\n%s",
                         binary.return_code, binary.stderr)
        return binary.return_code
    help_lines = binary.stdout.strip().splitlines()

    # beautify it before pasting into README
    while not help_lines[0].startswith("USAGE"):
        del help_lines[0]  # remove "About" line & other fluff
    del help_lines[0]  # remove "USAGE:" header
    flags = re.findall(r'--(?:\w|[-_])+', help_lines[0])
    help_lines[1:] = [
        # remove the description lines of flags that aren't in the usage string
        line for line in help_lines[1:]
        if not '--' in line or any(flag in line for flag in flags)
    ]
    help_lines[:1] = (
        # make the usage line more readable by splitting long flags into
        # separate lines, and fix binary name
        ']\n    '.join(help_lines[0].strip().split('] '))
        .replace("rush", "rh")
        .splitlines()
    )
    help = os.linesep.join('    ' + line for line in help_lines)

    # paste the modified help into README
    verbose and logging.info("Updating README to add binary's help string")
    with (Path.cwd() / 'README.md').open('r+t', encoding='utf-8') as f:
        readme_lines = [line.rstrip() for line in f.readlines()]

        # determine the line indices of the region to replace,
        # which is between the header titled "Usage" and the immediate next one
        begin_idx, end_idx = None, None
        for i, line in enumerate(readme_lines):
            if not line.startswith('#'):
                continue
            if begin_idx is None:
                if "# Usage" in line:
                    begin_idx = i
            else:
                end_idx = i
                break
        if begin_idx is None or end_idx is None:
            logging.critical("usage begin or end markers not found in README "
                             "(begin:%s, end:%s)", begin_idx, end_idx)
            return 2

        # reassemble the modified content of the README, with help inside
        readme_content = os.linesep.join([
            os.linesep.join(readme_lines[:begin_idx + 1]),
            '', help, '',
            os.linesep.join(readme_lines[end_idx:]),
            '',   # ensure newline at the end of file
        ])

        f.seek(0)
        f.truncate()
        f.write(readme_content)


@task(help=HELP)
def lib(ctx, release=False, verbose=False):
    """Build the library crate."""
    ensure_rustc_version(ctx)
    return cargo(
        ctx, 'build', *get_rustc_flags(release, verbose), crate=LIB, pty=True
    ).return_code


@task(name='docs', help=HELP)
def docs_(ctx, release=False, verbose=False, dump_api=False):
    """Build the project documentation.

    This includes analyzing the Rust modules that implement expression API
    in order to extract their in-code documentation before putting it in
    the dedicated documentation page as Markdown.

    It also removes some of the superfluous files from the docs output
    directory in release mode.
    """
    # describe the API modules and functions contained therein,
    # rendering the documentation as Markdown into the designated doc page
    is_root_mod_rs = lambda p: p.stem == 'mod' and p.parent.stem == 'api'
    module_paths = [
        mod for mod in Path('./crates', LIB, 'src/eval/api').rglob('**/*.rs')
        if not is_root_mod_rs(mod)]
    modules = docs.describe_rust_api(*module_paths)
    docs.insert_api_docs(modules, into='./docs/api.md')

    # build the docs in output format
    args = ['--strict']
    if release:
        args.append('--clean')
    if verbose:
        args.append('--verbose')
    build_run = ctx.run('mkdocs build ' + ' '.join(map(quote, args)), pty=True)
    if not build_run.ok:
        logging.fatal("mkdocs build failed, aborting.")
        return build_run.return_code

    mkdocs_config = docs.read_mkdocs_config()
    source_dir = Path.cwd() / mkdocs_config.get('docs_dir', 'docs')
    output_dir = Path.cwd() / mkdocs_config.get('site_dir', 'site')

    # purge any HTML comments that have been carried from Markdown
    for path in output_dir.rglob('*.html'):
        docs.scrub_html_comment_markers(path)

    # for release doc builds, clean some of the output files that get
    # copied verbatim since mkdocs doesn't support ignoring them
    if release:
        # read the list of ignored path patterns from a file
        ignored = []
        ignore_file = source_dir / '.docsignore'
        if ignore_file.exists():
            verbose and logging.info(
                "%s file found, applying ignore patterns...", ignore_file)
            with ignore_file.open(encoding='utf-8') as f:
                ignored = [
                    line.rstrip() for line in f.readlines()
                    if line.strip() and not line.lstrip().startswith('#')]
        else:
            verbose and logging.info(
                "%s not found, not removing any ignored files.", ignore_file)

        # resolve the patterns to see what files in the output dir
        # they correspond to, if any
        if ignored:
            ignored = chain.from_iterable(imap(output_dir.glob, ignored))

        # "ignore" them, i.e. delete from output directory
        for path in ignored:
            verbose and logging.info(
                "Removing ignored file/directory '%s'", path)
            if path.is_dir():
                shutil.rmtree(str(path))
            else:
                path.unlink()


# Utility functions

def ensure_rustc_version(ctx):
    """Terminates the build unless the Rust compiler is recent enough."""
    rustc_v = ctx.run('rustc --version', hide=True)
    if not rustc_v.ok:
        logging.critical("Rust compiler not found, aborting build.")
        sys.exit(rustc_v.return_code)

    _, version, _ = rustc_v.stdout.split(None, 2)
    if not semver.match(version, '>=' + MIN_RUSTC_VERSION):
        logging.error("Build requires at least Rust %s, found %s",
                      MIN_RUSTC_VERSION, version)
        sys.exit(1)

    return True


def get_rustc_flags(release, verbose):
    """Return a list of Rust compiler flags corresponding to given params."""
    flags = []
    if release:
        flags.append('--release')
    if verbose:
        flags.append('--verbose')
    return flags
