//! Data structures representing the abstract syntax tree (AST)
//! of parsed expressions.

use std::str::FromStr;

use eval::{self, Eval, EvalResult, Context, Value};


pub struct ValueNode {
    pub value: Value,
}

impl FromStr for ValueNode {
    type Err = <Value as FromStr>::Err;

    fn from_str(s: &str) -> Result<ValueNode, Self::Err> {
        s.parse::<Value>().map(|v| ValueNode{value: v})
    }
}

impl Eval for ValueNode {
    fn eval(&self, context: &Context) -> EvalResult {
        if let Value::String(ref string) = self.value {
            // treat the literal value as variable name if such variable exists;
            // otherwise, just return the value itself as string
            if let Some(value) = context.get_var(string) {
                return Ok(value.clone());
            }
            return Ok(Value::String(string.clone()));
        }
        Ok(self.value.clone())
    }
}


pub struct BinaryOpNode {
    pub first: Box<Eval>,
    pub rest: Vec<(String, Box<Eval>)>,
}

impl Eval for BinaryOpNode {
    fn eval(&self, context: &Context) -> EvalResult {
        let mut result = try!(self.first.eval(&context));
        for &(ref op, ref arg) in &self.rest {
            let arg = try!(arg.eval(&context));
            match &op[..] {
                "+" => result = try!(BinaryOpNode::eval_plus(&context, &result, &arg)),
                "-" => result = try!(BinaryOpNode::eval_minus(&context, &result, &arg)),
                // TODO(xion): other operators
                _ => { return eval::Error::err(&format!("unknown operator: {}", op)); }
            }
        }
        Ok(result)
    }
}

impl BinaryOpNode {
    /// Evaluate the "+" operator for two values.
    fn eval_plus(context: &Context, left: &Value, right: &Value) -> EvalResult {
        if let &Value::String(ref left) = left {
            if let &Value::String(ref right) = right {
                return Ok(Value::String(left.clone() + &*right));
            }
        }
        if let Value::Integer(left) = *left {
            if let Value::Integer(right) = *right {
                return Ok(Value::Integer(left + right));
            }
        }
        if let Value::Float(left) = *left {
            if let Value::Float(right) = *right {
                return Ok(Value::Float(left + right));
            }
        }
        eval::Error::err("invalid types for (+) operator")
    }

    /// Evaluate the "-" operator for two values.
    fn eval_minus(context: &Context, left: &Value, right: &Value) -> EvalResult {
        if let Value::Integer(left) = *left {
            if let Value::Integer(right) = *right {
                return Ok(Value::Integer(left - right));
            }
        }
        if let Value::Float(left) = *left {
            if let Value::Float(right) = *right {
                return Ok(Value::Float(left - right));
            }
        }
        eval::Error::err("invalid types for (-) operator")
    }
}


// // TODO(xion): change to general OperatorNode that has starting value
// // and arbitrary number of (op, value) pairs that it goes over during
// // evaluation (the parser shall take care of operator precedence while
// // building the tree)
// pub struct BinaryOpNode {
//     pub op: String,  // TODO(xion): enum?
//     pub left: Box<Eval>,
//     pub right: Box<Eval>,
// }

// impl Eval for BinaryOpNode {
//     fn eval(&self, context: &Context) -> Result<Value, eval::Error> {
//         match &self.op[..] {
//             "+" => {
//                 let left = try!(self.left.eval(&context));
//                 let right = try!(self.right.eval(&context));

//                 if let Value::String(left) = left {
//                     if let Value::String(right) = right {
//                         return Ok(Value::String(left + &right));
//                     }
//                 }
//                 // TODO(xion): adding numbers
//                 eval::Error::err("invalid types for + operator")
//             }
//             // TODO(xion): other operators
//             _ => eval::Error::err(&format!("unknown operator: {}", self.op))
//         }
//     }
// }


pub struct FunctionCallNode {
    pub name: String,
    pub args: Vec<Box<Eval>>,
}

impl Eval for FunctionCallNode {
    fn eval(&self, context: &Context) -> Result<Value, eval::Error> {
        // evaluate all the arguments first, bail if any of that fails
        let evals: Vec<_> =
            self.args.iter().map(|x| x.eval(&context)).collect();
        if let Some(res) = evals.iter().find(|r| r.is_err()) {
            return res.clone();
        }

        // extract the argument values and call the function
        let args = evals.iter().map(|r| r.clone().ok().unwrap()).collect();
        context.call_func(&self.name, args).ok_or(
            eval::Error{message: format!("unknown function: {}", self.name)}
        )
    }
}
