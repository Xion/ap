//! Module implementing the evaluation of postfix operators.

use ast::{FunctionCallNode, SubscriptNode};

use eval::{self, Context, Eval, Value};


/// Evaluate the function call AST node.
impl Eval for FunctionCallNode {
    fn eval(&self, context: &Context) -> eval::Result {
        // evaluate all the arguments first, bail if any of that fails
        let evals: Vec<_> =
            self.args.iter().map(|x| x.eval(&context)).collect();
        if let Some(res) = evals.iter().find(|r| r.is_err()) {
            return res.clone();
        }

        // extract the argument values and call the function
        let args = evals.iter().map(|r| r.clone().ok().unwrap()).collect();
        context.call_func(&self.name, args)
    }
}


/// Evaluate the array subscripting AST node.
impl Eval for SubscriptNode {
    fn eval(&self, context: &Context) -> eval::Result {
        let object = try!(self.object.eval(&context));
        let index = try!(self.index.eval(&context));

        match object {
            Value::Array(ref a) => SubscriptNode::eval_on_array(&a, &index),
            Value::String(ref s) => SubscriptNode::eval_on_string(&s, &index),
            _ => Err(eval::Error::new(
                &format!("can't index {:?} with {:?}", object, index)
            )),
        }
    }
}
impl SubscriptNode {
    // TODO(xion): consider supporting Python-style negative indices

    fn eval_on_array(array: &Vec<Value>, index: &Value) -> eval::Result {
        match *index {
            Value::Integer(i) => {
                if i < 0 {
                    return Err(eval::Error::new(
                        &format!("array index cannot be negative; got {}", i)
                    ));
                }
                let idx = i as usize;
                if idx >= array.len() {
                    return Err(eval::Error::new(
                        &format!("array index out of range ({})", i)
                    ));
                }
                // TODO(xion): the clone below is very inefficient for
                // multi-dimensional arrays; return some Value pointer instead
                Ok(array[idx].clone())
            },
            Value::Float(..) => Err(eval::Error::new(
                &format!("array indices must be integers")
            )),
            _ => Err(eval::Error::new(
                &format!("can't index an array with {:?}", index)
            )),
        }
    }

    fn eval_on_string(string: &String, index: &Value) -> eval::Result {
        match *index {
            Value::Integer(i) => {
                string.chars().nth(i as usize)
                    .ok_or_else(|| eval::Error::new(
                        &format!("character index out of range: {}", i)
                    ))
                    .map(|c| {
                        let mut result = String::new();
                        result.push(c);
                        Value::String(result)
                    })
            },
            _ => Err(eval::Error::new(
                &format!("can't index a string with {:?}", index)
            )),
        }
    }
}
