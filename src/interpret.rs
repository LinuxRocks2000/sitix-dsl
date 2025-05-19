// impls for all of the ast types that give them the interpret() function

use crate::ast::*;
use thiserror::Error;


#[derive(Debug, PartialEq)]
pub enum Data { // data is the *interpreter's* idea of Sitix data.
    Boolean(bool),
    Nil, // the standard return type
    Number(f64),
    String(String),
    Sitix(String), // this is a fairly magical high-level builtin type. it is the result of evaluating
                   // a SitixExpression.
                   // TODO: handle properties (this should eventually be (String, HashMap<String, Data>), once we've implemented that)
    // TODO: tables and other abstract datatypes
}

impl ToString for Data {
    fn to_string(&self) -> String {
        match self {
            Self::Boolean(b) => if *b { "true".to_string() } else { "false".to_string() },
            Self::Nil => "".to_string(),
            Self::Number(n) => n.to_string(),
            Self::String(s) => s.clone(),
            Self::Sitix(s) => s.clone()
        }
    }
}


impl Data {
    pub fn force_boolean(&self) -> InterpretResult<bool> {
        if let Self::Boolean(data) = self {
            Ok(*data)
        }
        else {
            Err(InterpretError::InvalidType(format!("expected bool, got {:?}", self)))
        }
    }

    pub fn force_number(&self) -> InterpretResult<f64> {
        if let Self::Number(data) = self {
            Ok(*data)
        }
        else {
            Err(InterpretError::InvalidType(format!("expected number, got {:?}", self)))
        }
    }
}


pub struct InterpreterState {

}

impl InterpreterState {
    pub fn new() -> Self {
        Self {}
    }
}

#[derive(Error, Debug)]
pub enum InterpretError {
    #[error("Invalid type: {0}")]
    InvalidType(String)
}

type InterpretResult<T> = Result<T, InterpretError>;

impl SitixExpression {
    pub fn interpret(&self, interpreter : &mut InterpreterState) -> InterpretResult<Data> {
        Ok(match self {
            Self::Block(b) => {
                Data::Sitix(b.interpret(interpreter)?.to_string())
            },
            Self::Text(text) => Data::Sitix(text.clone())
        })
    }
}

impl Block {
    fn interpret(&self, i : &mut InterpreterState) -> InterpretResult<Data> {
        for statement in &self.inner {
            statement.interpret(i); // throw away the result
        }
        if let Some(tail) = &self.tail {
            tail.interpret(i)
        }
        else {
            Ok(Data::Nil)
        }
    }
}

impl Statement {
    fn interpret(&self, i : &mut InterpreterState) -> InterpretResult<Data> {
        match self {
            Self::Expression(expr) => expr.interpret(i)
        }
    }
}

impl Expression {
    fn interpret(&self, i : &mut InterpreterState) -> InterpretResult<Data> {
        match self {
            Self::Literal(l) => l.interpret(i),
            Self::Unary(u) => u.interpret(i),
            Self::Binary(b) => b.interpret(i),
            Self::Grouping(e) => e.interpret(i),
            Self::Braced(b) => todo!("braced expressions"),
            Self::SitixExpression(v) => {
                let mut result = String::new();
                for expr in v {
                    result += &expr.interpret(i)?.to_string();
                }
                Ok(Data::Sitix(result))
            },
            Self::True => Ok(Data::Boolean(true)),
            Self::False => Ok(Data::Boolean(false)),
            Self::Nil => Ok(Data::Boolean(false))
        }
    }
}

impl Unary {
    fn interpret(&self, i : &mut InterpreterState) -> InterpretResult<Data> {
        Ok(match self {
            Self::Negative(expr) => {
                let res = expr.interpret(i)?;
                Data::Number(res.force_number()? * -1.0)
            },
            Self::Not(expr) => {
                let res = expr.interpret(i)?;
                Data::Boolean(!(res.force_boolean()?))
            }
        })
    }
}

impl Literal {
    fn interpret(&self, i : &mut InterpreterState) -> InterpretResult<Data> {
        Ok(match self {
            Self::Ident(_) => todo!("identifier lookup"),
            Self::String(s) => Data::String(s.clone()),
            Self::Text(s) => Data::Sitix(s.clone()),
            Self::Number(n) => Data::Number(*n)
        })
    }
}

impl Binary {
    fn interpret(&self, i : &mut InterpreterState) -> InterpretResult<Data> {
        Ok(match self {
            Self::Equals(one, two) => {
                let one = one.interpret(i)?;
                let two = two.interpret(i)?;
                Data::Boolean(one == two)
            },
            Self::Nequals(one, two) => {
                let one = one.interpret(i)?;
                let two = two.interpret(i)?;
                Data::Boolean(one != two)
            },
            Self::And(one, two) => {
                let one = one.interpret(i)?.force_boolean()?;
                let two = two.interpret(i)?.force_boolean()?;
                Data::Boolean(one && two)
            },
            Self::Or(one, two) => {
                let one = one.interpret(i)?.force_boolean()?;
                let two = two.interpret(i)?.force_boolean()?;
                Data::Boolean(one || two)
            },
            Self::Add(one, two) => {
                let one = one.interpret(i)?;
                let two = two.interpret(i)?;
                if let Data::String(s) = one {
                    Data::String(s + &two.to_string())
                }
                else if let Data::String(s) = two {
                    Data::String(one.to_string() + &s)
                }
                else if let Data::Sitix(s) = one {
                    Data::String(s + &two.to_string())
                }
                else if let Data::Sitix(s) = two {
                    Data::String(one.to_string() + &s)
                }
                else {
                    let one = one.force_number()?;
                    let two = two.force_number()?;
                    Data::Number(one + two)
                }
            },
            Self::Sub(one, two) => {
                let one = one.interpret(i)?.force_number()?;
                let two = two.interpret(i)?.force_number()?;
                Data::Number(one - two)
            },
            Self::Mul(one, two) => {
                let one = one.interpret(i)?.force_number()?;
                let two = two.interpret(i)?.force_number()?;
                Data::Number(one * two)
            },
            Self::Div(one, two) => {
                let one = one.interpret(i)?.force_number()?;
                let two = two.interpret(i)?.force_number()?;
                Data::Number(one / two)
            },
            Self::Gt(one, two) => {
                let one = one.interpret(i)?.force_number()?;
                let two = two.interpret(i)?.force_number()?;
                Data::Boolean(one > two)
            },
            Self::Gte(one, two) => {
                let one = one.interpret(i)?.force_number()?;
                let two = two.interpret(i)?.force_number()?;
                Data::Boolean(one >= two)
            },
            Self::Lt(one, two) => {
                let one = one.interpret(i)?.force_number()?;
                let two = two.interpret(i)?.force_number()?;
                Data::Boolean(one < two)
            },
            Self::Lte(one, two) => {
                let one = one.interpret(i)?.force_number()?;
                let two = two.interpret(i)?.force_number()?;
                Data::Boolean(one <= two)
            }
        })
    }
}
