//! Module with built-in API that's available to the expressions.
//! This is basically the standard library of the language.

// NOTE: All actual API functions should be defined in submodules.

pub mod base;
pub mod conv;
pub mod math;
pub mod strings;


use std::borrow::{Borrow, ToOwned};
use std::f64;
use std::fmt::Display;
use std::hash::Hash;

use eval::{self, Context, Error, Value};
use eval::model::{Args, Arity, Function, Name};
use eval::value::FloatRepr;


impl<'c> Context<'c> {
    /// Initialize symbols for the built-in functions and constants.
    /// This should be done only for the root Context (the one w/o a parent).
    pub fn init_builtins(&mut self) {
        assert!(self.is_root(), "Only root Context can have builtins!");
        self.init_functions();
        self.init_constants();
    }

    fn init_functions(&mut self) {
        //
        // Keep the list sorted alphabetically by function names.
        //
        self.define_unary(          "abs",      math::abs           );
        self.define_binary(         "after",    strings::after      );
        self.define_unary(          "all",      base::all           );
        self.define_unary(          "any",      base::any           );
        self.define_binary(         "before",   strings::before     );
        self.define_unary(          "bin",      math::bin           );
        self.define_unary(          "bool",     conv::bool          );
        self.define_unary(          "ceil",     math::ceil          );
        self.define_unary(          "char",     strings::chr        );
        self.define_unary(          "chr",      strings::chr        );
        self.define_unary(          "csv",      conv::csv           );
        self.define_unary(          "exp",      math::exp           );
        self.define_binary_ctx(     "filter",   base::filter        );
        self.define_unary(          "float",    conv::float         );
        self.define_unary(          "floor",    math::floor         );
        self.define_ternary_ctx(    "fold",     base::reduce        );
        self.define_ternary_ctx(    "foldl",    base::reduce        );
        self.define_binary(         "format",   strings::format_    );
        self.define_ternary_ctx(    "gsub",     strings::sub        );
        self.define_unary(          "hex",      math::hex           );
        self.define_binary(         "index",    base::index         );
        self.define_unary(          "int",      conv::int           );
        self.define_binary(         "join",     strings::join       );
        self.define_unary(          "json",     conv::json          );
        self.define_unary(          "len",      base::len           );
        self.define_unary(          "ln",       math::ln            );
        self.define_binary_ctx(     "map",      base::map           );
        self.define_unary_ctx(      "max",      base::max           );
        self.define_unary_ctx(      "min",      base::min           );
        self.define_unary(          "oct",      math::oct           );
        self.define_unary(          "ord",      strings::ord        );
        self.define_nullary(        "rand",     math::rand          );
        self.define_unary(          "re",       conv::regex         );
        self.define_ternary_ctx(    "reduce",   base::reduce        );
        self.define_unary(          "regex",    conv::regex         );
        self.define_unary(          "regexp",   conv::regex         );
        self.define_unary(          "rev",      strings::rev        );
        self.define_unary(          "rot13",    strings::rot13      );
        self.define_unary(          "round",    math::round         );
        self.define_ternary(        "rsub1",    strings::rsub1      );
        self.define_unary(          "sgn",      math::sgn           );
        self.define_unary(          "sort",     base::sort          );
        self.define_binary_ctx(     "sortby",   base::sort_by       );
        self.define_binary(         "split",    strings::split      );
        self.define_unary(          "sqrt",     math::sqrt          );
        self.define_unary(          "str",      conv::str_          );
        self.define_ternary_ctx(    "sub",      strings::sub        );
        self.define_ternary_ctx(    "sub1",     strings::sub1       );
        self.define_unary_ctx(      "sum",      base::sum           );
        self.define_unary(          "trim",     strings::trim       );
        self.define_unary(          "trunc",    math::trunc         );
    }

    fn init_constants(&mut self) {
        //
        // Keep the list sorted alphabetically by constant names (ignore case).
        //
        self.set(   "Inf",      Value::Float(f64::INFINITY as FloatRepr)    );
        self.set(   "NaN",      Value::Float(f64::NAN as FloatRepr)         );
        self.set(   "nil",      Value::Empty                                );
        self.set(   "pi",       Value::Float(f64::consts::PI as FloatRepr)  );
    }
}


// Helper methods for defining the "pure" API functions
// (those that don't access the Context directly).
#[allow(dead_code)]
impl<'c> Context<'c> {
    fn define<'n, N: ?Sized, F>(&mut self, name: &'static N, arity: Arity, func: F) -> &mut Self
        where Name: Borrow<N>, N: ToOwned<Owned=Name> + Hash + Eq + Display,
              F: Fn(Args) -> eval::Result + 'static
    {
        assert!(!self.is_defined_here(name),
             "`{}` has already been defined in this Context!", name);

        let function = Function::from_native(arity, move |args: Args| {
            try!(ensure_argcount(name, &args, arity));
            func(args)
        });
        self.set(name, Value::Function(function));
        self
    }

    fn define_nullary<N:? Sized, F>(&mut self, name: &'static N, func: F) -> &mut Self
        where Name: Borrow<N>, N: ToOwned<Owned=Name> + Hash + Eq + Display,
              F: Fn() -> eval::Result + 'static
    {
        self.define(name, Arity::Exact(0), move |_| { func() })
    }
    fn define_nullary_plus<N: ?Sized, F>(&mut self, name: &'static N, func: F) -> &mut Self
        where Name: Borrow<N>, N: ToOwned<Owned=Name> + Hash + Eq + Display,
              F: Fn(Args) -> eval::Result + 'static
    {
        self.define(name, Arity::Minimum(0), func)
    }

    fn define_unary<N: ?Sized, F>(&mut self, name: &'static N, func: F) -> &mut Self
        where Name: Borrow<N>, N: ToOwned<Owned=Name> + Hash + Eq + Display,
              F: Fn(Value) -> eval::Result + 'static
    {
        self.define(name, Arity::Exact(1), move |args: Args| {
            let mut args = args.into_iter();
            func(args.next().unwrap())
        })
    }
    fn define_unary_plus<N: ?Sized, F>(&mut self, name: &'static N, func: F) -> &mut Self
        where Name: Borrow<N>, N: ToOwned<Owned=Name> + Hash + Eq + Display,
              F: Fn(Args) -> eval::Result + 'static
    {
        self.define(name, Arity::Minimum(1), func)
    }

    fn define_binary<N: ?Sized, F>(&mut self, name: &'static N, func: F) -> &mut Self
        where Name: Borrow<N>, N: ToOwned<Owned=Name> + Hash + Eq + Display,
              F: Fn(Value, Value) -> eval::Result + 'static
    {
        self.define(name, Arity::Exact(2), move |args: Args| {
            let mut args = args.into_iter();
            func(args.next().unwrap(), args.next().unwrap())
        })
    }
    fn define_binary_plus<N: ?Sized, F>(&mut self, name: &'static N, func: F) -> &mut Self
        where Name: Borrow<N>, N: ToOwned<Owned=Name> + Hash + Eq + Display,
              F: Fn(Args) -> eval::Result + 'static
    {
        self.define(name, Arity::Minimum(2), func)
    }

    fn define_ternary<N: ?Sized, F>(&mut self, name: &'static N, func: F) -> &mut Self
        where Name: Borrow<N>, N: ToOwned<Owned=Name> + Hash + Eq + Display,
              F: Fn(Value, Value, Value) -> eval::Result + 'static
    {
        self.define(name, Arity::Exact(3), move |args: Args| {
            let mut args = args.into_iter();
            func(args.next().unwrap(),
                 args.next().unwrap(),
                 args.next().unwrap())
        })
    }
    fn define_ternary_plus<N: ?Sized, F>(&mut self, name: &'static N, func: F) -> &mut Self
        where Name: Borrow<N>, N: ToOwned<Owned=Name> + Hash + Eq + Display,
              F: Fn(Args) -> eval::Result + 'static
    {
        self.define(name, Arity::Minimum(3), func)
    }
}

// Helper methods for defining the API functions which access the Context.
#[allow(dead_code)]
impl<'c> Context<'c> {
    fn define_ctx<N: ?Sized, F>(&mut self, name: &'static N, arity: Arity, func: F) -> &mut Self
        where Name: Borrow<N>, N: ToOwned<Owned=Name> + Hash + Eq + Display,
              F: Fn(Args, &Context) -> eval::Result + 'static
    {
        assert!(!self.is_defined_here(name),
             "`{}` has already been defined in this Context!", name);

        let function = Function::from_native_ctx(arity, move |args: Args, context: &Context| {
            try!(ensure_argcount(name, &args, arity));
            func(args, &context)
        });
        self.set(name, Value::Function(function));
        self
    }

    fn define_nullary_ctx<N: ?Sized, F>(&mut self, name: &'static N, func: F) -> &mut Self
        where Name: Borrow<N>, N: ToOwned<Owned=Name> + Hash + Eq + Display,
              F: Fn(&Context) -> eval::Result + 'static
    {
        self.define_ctx(name, Arity::Exact(0), move |_, context: &Context| {
            func(&context)
        })
    }
    fn define_nullary_plus_ctx<N: ?Sized, F>(&mut self, name: &'static N, func: F) -> &mut Self
        where Name: Borrow<N>, N: ToOwned<Owned=Name> + Hash + Eq + Display,
              F: Fn(Args, &Context) -> eval::Result + 'static
    {
        self.define_ctx(name, Arity::Minimum(0), func)
    }

    fn define_unary_ctx<N: ?Sized, F>(&mut self, name: &'static N, func: F) -> &mut Self
        where Name: Borrow<N>, N: ToOwned<Owned=Name> + Hash + Eq + Display,
              F: Fn(Value, &Context) -> eval::Result + 'static
    {
        self.define_ctx(name, Arity::Exact(1), move |args: Args, context: &Context| {
            let mut args = args.into_iter();
            func(args.next().unwrap(), &context)
        })
    }
    fn define_unary_plus_ctx<N: ?Sized, F>(&mut self, name: &'static N, func: F) -> &mut Self
        where Name: Borrow<N>, N: ToOwned<Owned=Name> + Hash + Eq + Display,
              F: Fn(Args, &Context) -> eval::Result + 'static
    {
        self.define_ctx(name, Arity::Minimum(1), func)
    }

    fn define_binary_ctx<N: ?Sized, F>(&mut self, name: &'static N, func: F) -> &mut Self
        where Name: Borrow<N>, N: ToOwned<Owned=Name> + Hash + Eq + Display,
              F: Fn(Value, Value, &Context) -> eval::Result + 'static
    {
        self.define_ctx(name, Arity::Exact(2), move |args: Args, context: &Context| {
            let mut args = args.into_iter();
            func(args.next().unwrap(), args.next().unwrap(),
                &context)
        })
    }
    fn define_binary_plus_ctx<N: ?Sized, F>(&mut self, name: &'static N, func: F) -> &mut Self
        where Name: Borrow<N>, N: ToOwned<Owned=Name> + Hash + Eq + Display,
              F: Fn(Args, &Context) -> eval::Result + 'static
    {
        self.define_ctx(name, Arity::Minimum(2), func)
    }

    fn define_ternary_ctx<N: ?Sized, F>(&mut self, name: &'static N, func: F) -> &mut Self
        where Name: Borrow<N>, N: ToOwned<Owned=Name> + Hash + Eq + Display,
              F: Fn(Value, Value, Value, &Context) -> eval::Result + 'static
    {
        self.define_ctx(name, Arity::Exact(3), move |args: Args, context: &Context| {
            let mut args = args.into_iter();
            func(args.next().unwrap(),
                 args.next().unwrap(),
                 args.next().unwrap(),
                 &context)
        })
    }
    fn define_ternary_plus_ctx<N: ?Sized, F>(&mut self, name: &'static N, func: F) -> &mut Self
        where Name: Borrow<N>, N: ToOwned<Owned=Name> + Hash + Eq + Display,
              F: Fn(Args, &Context) -> eval::Result + 'static
    {
        self.define_ctx(name, Arity::Minimum(3), func)
    }
}


/// Make sure a function got the correct number of arguments.
/// Usage:
///     try!(ensure_argcount("function", min, max));
///
fn ensure_argcount<N: ?Sized>(name: &N, args: &Args, arity: Arity) -> Result<(), Error>
    where N: Display
{
    let count = args.len();
    if arity.accepts(count) {
        Ok(())
    } else {
        Err(Error::new(&format!(
            "invalid number of arguments to {}(): expected {}, got {}",
            name, arity, count
        )))
    }
}


#[cfg(test)]
mod tests {
    use eval::Context;

    #[test]
    fn no_bool_constants() {
        let ctx = Context::new();
        for constant in &["true", "false"] {
            assert!(!ctx.is_defined(*constant),
                "`{}` is handled by parser and doesn't need to be in Context", constant);
        }
    }
}