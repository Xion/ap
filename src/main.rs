//! Module with the entry point of the binary.

extern crate getopts;
#[macro_use]
extern crate log;

extern crate ap;


mod logging;


use std::env;
use std::io::{self, Write};
use std::process::exit;

use getopts::Options;


fn main() {
    logging::init().unwrap();

    let argv: Vec<String> = env::args().collect();
    let program = argv[0].clone();

    let mut options = Options::new();
    options.optflag("h", "help", "Show this usage message");
    options.optflag("p", "parse", "Only parse the expression, printing AST");
    // TODO(xion): -l flag that causes input to be interpreted as single array of lines
    // rather make the expression execute againts each line individually

    let args = options.parse(&argv[1..]).unwrap();
    if args.opt_present("h") {
        print_usage(&program, options);
        return;
    }

    if args.free.len() != 1 {
        error!("Invalid number of expression arguments; expected 1, got {}",
               args.free.len());
        exit(1);
    }

    let expr = &args.free[0];
    if args.opt_present("p") {
        match ap::parse(expr) {
            Ok(ast) => println!("{:?}", ast),
            Err(error) => { error!("{:?}", error); exit(1); },
        }
    } else {
        if let Err(error) = ap::apply(expr, io::stdin(), &mut io::stdout()) {
            error!("{:?}", error);
            exit(1);
        }
    }
}


/// Print the instructions about invoking the program from the command line.
fn print_usage(program: &str, opts: Options) {
    writeln!(&mut io::stderr(), "{}",
             opts.usage(&format!("Usage: {} [options]", program))).unwrap();
}
