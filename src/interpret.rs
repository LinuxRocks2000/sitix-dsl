// impls for all of the ast types that give them the interpret() function

use crate::ast::*;
use thiserror::Error;
use std::collections::HashMap;


#[derive(Debug, PartialEq, Clone)]
pub enum Data { // data is the *interpreter's* idea of Sitix data.
    Boolean(bool),
    Nil, // the standard return type
    Number(f64),
    String(String),
    VariableHandle(usize),
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
            Self::Sitix(s) => s.clone(),
            Self::VariableHandle(u) => format!("variable handle {}", u)
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


#[derive(Debug)]
pub struct InterpreterState {
    variables : Vec<Data>,
    scopes : Vec<HashMap<String, usize>> // ne'er on stranger shores has found/a data structure more cursed than this
}


impl InterpreterState {
    pub fn new() -> Self {
        Self {
            variables : vec![],
            scopes : vec![HashMap::new()] // global scope
        }
    }

    pub fn get(&self, name : &String) -> InterpretResult<Data> {
        for scope in self.scopes.iter().rev() {
            if let Some(index) = scope.get(name) {
                return Ok(Data::VariableHandle(*index));
            }
        }
        Err(InterpretError::UndefinedSymbol(name.clone()))
    }

    pub fn create(&mut self, name : String, data : Data) -> Data {
        self.variables.push(data);
        self.scopes.last_mut().unwrap().insert(name, self.variables.len() - 1);
        Data::VariableHandle(self.variables.len() - 1)
    }

    pub fn create_global(&mut self, name : String, data : Data) -> Data {
        self.variables.push(data);
        self.scopes.first_mut().unwrap().insert(name, self.variables.len() - 1);
        Data::VariableHandle(self.variables.len() - 1)
    }

    pub fn set(&mut self, handle : Data, data : Data) -> InterpretResult<()> {
        if let Data::VariableHandle(u) = handle {
            if let Some(var) = self.variables.get_mut(u) {
                *var = data;
                Ok(())
            }
            else {
                Err(InterpretError::BadHandle)
            }
        }
        else {
            Err(InterpretError::InvalidType("Expected variable".to_string()))
        }
    }

    pub fn open_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn close_scope(&mut self) {
        self.scopes.pop();
    }

    pub fn deref(&self, data : Data) -> InterpretResult<Data> {
        match data {
            Data::VariableHandle(index) => self.variables.get(index).cloned().ok_or(InterpretError::BadHandle),
            _ => Ok(data)
        }
    }
}

#[derive(Error, Debug)]
pub enum InterpretError {
    #[error("Invalid type: {0}")]
    InvalidType(String),
    #[error("{0} has not been defined in this scope")]
    UndefinedSymbol(String),
    #[error("Bad variable handle")]
    BadHandle
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
        i.open_scope();
        for statement in &self.inner {
            statement.interpret(i)?; // throw away the result
        }
        if let Some(tail) = &self.tail {
            let out = tail.interpret(i)?;
            let out = i.deref(out)?;
            i.close_scope();
            Ok(out)
        }
        else {
            i.close_scope();
            Ok(Data::Nil)
        }
    }
}

impl Statement {
    fn interpret(&self, i : &mut InterpreterState) -> InterpretResult<Data> {
        match self {
            Self::Expression(expr) => expr.interpret(i),
            Self::Print(expr) => {
                println!("{}", expr.interpret(i)?.to_string());
                Ok(Data::Nil)
            },
            Self::LetAssign(ident, expr) => {
                let value = expr.interpret(i)?;
                i.create(ident.clone(), value);
                Ok(Data::Nil)
            },
            Self::GlobalAssign(ident, expr) => {
                let value = expr.interpret(i)?;
                i.create_global(ident.clone(), value);
                Ok(Data::Nil)
            },
            Self::Debugger => {
                println!("==DEBUGGER==\nstate is {:?}", i);
                Ok(Data::Nil)
            }
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
            Self::Braced(b) => b.interpret(i),
            Self::SitixExpression(v) => {
                let mut result = String::new();
                for expr in v {
                    let r = expr.interpret(i)?;
                    result += &i.deref(r)?.to_string();
                }
                Ok(Data::Sitix(result))
            },
            Self::True => Ok(Data::Boolean(true)),
            Self::False => Ok(Data::Boolean(false)),
            Self::Nil => Ok(Data::Nil),
            Self::VariableAccess(name) => {
                i.get(name)
            },
            Self::Assignment(variable, value) => {
                let var = variable.interpret(i)?;
                let value = value.interpret(i)?;
                i.set(var, i.deref(value.clone())?)?;
                Ok(value)
            }
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
    fn interpret(&self, _ : &mut InterpreterState) -> InterpretResult<Data> {
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
                let one = i.deref(one)?;
                let two = i.deref(two)?;
                Data::Boolean(one == two)
            },
            Self::Nequals(one, two) => {
                let one = one.interpret(i)?;
                let two = two.interpret(i)?;
                let one = i.deref(one)?;
                let two = i.deref(two)?;
                Data::Boolean(one != two)
            },
            Self::And(one, two) => {
                let one = one.interpret(i)?;
                let two = two.interpret(i)?;
                let one = i.deref(one)?.force_boolean()?;
                let two = i.deref(two)?.force_boolean()?;
                Data::Boolean(one && two)
            },
            Self::Or(one, two) => {
                let one = one.interpret(i)?;
                let two = two.interpret(i)?;
                let one = i.deref(one)?.force_boolean()?;
                let two = i.deref(two)?.force_boolean()?;
                Data::Boolean(one || two)
            },
            Self::Add(one, two) => {
                let one = one.interpret(i)?;
                let two = two.interpret(i)?;
                let one = i.deref(one)?;
                let two = i.deref(two)?;
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
                let one = one.interpret(i)?;
                let two = two.interpret(i)?;
                let one = i.deref(one)?.force_number()?;
                let two = i.deref(two)?.force_number()?;
                Data::Number(one - two)
            },
            Self::Mul(one, two) => {
                let one = one.interpret(i)?;
                let two = two.interpret(i)?;
                let one = i.deref(one)?.force_number()?;
                let two = i.deref(two)?.force_number()?;
                Data::Number(one * two)
            },
            Self::Div(one, two) => {
                let one = one.interpret(i)?;
                let two = two.interpret(i)?;
                let one = i.deref(one)?.force_number()?;
                let two = i.deref(two)?.force_number()?;
                Data::Number(one / two)
            },
            Self::Gt(one, two) => {
                let one = one.interpret(i)?;
                let two = two.interpret(i)?;
                let one = i.deref(one)?.force_number()?;
                let two = i.deref(two)?.force_number()?;
                Data::Boolean(one > two)
            },
            Self::Gte(one, two) => {
                let one = one.interpret(i)?;
                let two = two.interpret(i)?;
                let one = i.deref(one)?.force_number()?;
                let two = i.deref(two)?.force_number()?;
                Data::Boolean(one >= two)
            },
            Self::Lt(one, two) => {
                let one = one.interpret(i)?;
                let two = two.interpret(i)?;
                let one = i.deref(one)?.force_number()?;
                let two = i.deref(two)?.force_number()?;
                Data::Boolean(one < two)
            },
            Self::Lte(one, two) => {
                let one = one.interpret(i)?;
                let two = two.interpret(i)?;
                let one = i.deref(one)?.force_number()?;
                let two = i.deref(two)?.force_number()?;
                Data::Boolean(one <= two)
            }
        })
    }
}
