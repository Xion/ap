//! API that's available out-of-the-box to the expressions.
//! It is essentially the standard library of the language.

use rand::random;

use eval::{self, Error};
use super::model::Value;


/// Compute the length of given value (an array or a string).
pub fn len(value: Value) -> eval::Result {
    eval1!((value: &String) -> Integer { value.len() as i64 });
    eval1!((value: &Array) -> Integer { value.len() as i64 });
    Err(Error::new(&format!(
        "len() requires string or array, got {}", value.typename()
    )))
}

/// Compute the absolute value of a number.
pub fn abs(value: Value) -> eval::Result {
    eval1!(value : Integer { value.abs() });
    eval1!(value : Float { value.abs() });
    Err(Error::new(&format!(
        "abs() requires a number, got {}", value.typename()
    )))
}

/// Compute the signum function.
pub fn sgn(value : Value) -> eval::Result {
    eval1!(value : Integer {
        match value {
            v@_ if v < 0 => -1,
            v@_ if v > 0 => 1,
            _ => 0,
        }
    });
    eval1!(value : Float {
       match value {
            v@_ if v < 0.0 => -1.0,
            v@_ if v > 0.0 => 1.0,
            _ => 0.0,
        }
    });
    Err(Error::new(&format!(
        "sgn() requires a number, got {}", value.typename()
    )))
}

/// Generate a random floating point number from the 0..1 range.
pub fn rand() -> eval::Result {
    Ok(Value::Float(random()))
}


// Conversions

/// Convert a value to string.
pub fn str_(value: Value) -> eval::Result {
    match value {
        Value::String(_) => Ok(value),
        Value::Integer(i) => Ok(Value::String(i.to_string())),
        Value::Float(f) => Ok(Value::String(f.to_string())),
        Value::Boolean(b) => Ok(Value::String((
            if b { "true" } else { "false" }
        ).to_string())),
        _ => Err(Error::new(
            &format!("cannot convert {} to string", value.typename())
        )),
    }
}

/// Convert a value to an integer.
pub fn int(value: Value) -> eval::Result {
    match value {
        Value::String(ref s) => s.parse::<i64>()
            .map_err(|_| Error::new(&format!("invalid integer value: {}", s)))
            .map(Value::Integer),
        Value::Integer(_) => Ok(value),
        Value::Float(f) => Ok(Value::Integer(f as i64)),
        Value::Boolean(b) => Ok(Value::Integer(if b { 1 } else { 0 })),
        _ => Err(Error::new(
            &format!("cannot convert {} to int", value.typename())
        )),
    }
}

/// Convert a value to a float.
pub fn float(value: Value) -> eval::Result {
    match value {
        Value::String(ref s) => s.parse::<f64>()
            .map_err(|_| Error::new(&format!("invalid float value: {}", s)))
            .map(Value::Float),
        Value::Integer(i) => Ok(Value::Float(i as f64)),
        Value::Float(_) => Ok(value),
        Value::Boolean(b) => Ok(Value::Float(if b { 1.0 } else { 0.0 })),
        _ => Err(Error::new(
            &format!("cannot convert {} to float", value.typename())
        )),
    }
}

/// Convert a value to a boolean, based on its "truthy" value.
pub fn bool(value: Value) -> eval::Result {
    match value {
        Value::String(ref s) => s.parse::<bool>()
            .map_err(|_| Error::new(&format!("invalid bool value: {}", s)))
            .map(Value::Boolean),
        Value::Integer(i) => Ok(Value::Boolean(i != 0)),
        Value::Float(f) => Ok(Value::Boolean(f != 0.0)),
        Value::Boolean(_) => Ok(value),
        Value::Array(ref a) => Ok(Value::Boolean(a.len() > 0)),
        _ => Err(Error::new(
            &format!("cannot convert {} to bool", value.typename())
        )),
    }
}


// String functions

/// Reverse the character in a string.
pub fn rev(string: Value) -> eval::Result {
    // TODO(xion): since this reverses chars not graphemes,
    // it mangles some non-Latin strings;
    // fix with unicode-segmentation crate
    eval1!(string : &String { string.chars().rev().collect() });
    Err(Error::new(&format!(
        "rev() requires a string, got {}", string.typename()
    )))
}

/// Split a string by given string delimiter.
/// Returns an array of strings.
pub fn split(string: Value, delim: Value) -> eval::Result {
    eval2!((string: &String, delim: &String) -> Array {
        string.split(delim).map(str::to_owned).map(Value::String).collect()
    });
    Err(Error::new(&format!(
        "split() expects two strings, got: {}, {}",
        string.typename(), delim.typename()
    )))
}

/// Join an array of values into a single delimited string.
pub fn join(array: Value, delim: Value) -> eval::Result {
    if let (&Value::Array(ref a),
            &Value::String(ref d)) = (&array, &delim) {
        let strings: Vec<_> =  a.iter()
            .map(|v| str_(v.clone())).filter(Result::is_ok)
            .map(Result::unwrap).map(Value::unwrap_string)
            .collect();
        let error_count = strings.len() - a.len();
        if error_count == 0 {
            return Ok(Value::String(strings.join(&d)));
        } else {
            return Err(Error::new(&format!(
                "join() failed to stringify {} element(s) of the input array",
                error_count)));
        }
    }
    Err(Error::new(&format!(
        "join() expects an array and string, got: {}, {}",
        array.typename(), delim.typename()
    )))
}

/// Substitute a given string ("needle") with another ("replacement")
/// within given text ("haystack").
/// Returns the text after substitution has been made.
// TODO(xion): allow this function to accept just two arguments,
// with the third one being an implicit reference to the default var
// (requires allowing functions to access the Context)
pub fn sub(needle: Value, replacement: Value, haystack: Value) -> eval::Result {
    if let (&Value::String(ref n),
            &Value::String(ref r),
            &Value::String(ref h)) = (&needle, &replacement, &haystack) {
        return Ok(Value::String(h.replace(n, r)));
    }
    Err(Error::new(&format!(
        "sub() expects three strings, got: {}, {}, {}",
        needle.typename(), replacement.typename(), haystack.typename()
    )))
}
