// impls for all of the ast types that give them the interpret() function

use crate::ast::*;
use thiserror::Error;
use std::collections::HashMap;
use std::cell::RefCell;
use std::sync::Arc;


#[derive(Clone)]
pub enum SitixFunction {
    Builtin(&'static dyn Fn(&mut InterpreterState, &[Data]) -> InterpretResult<Data>),
    UserDefined(Vec<String>, Box<Expression>)
}


impl std::fmt::Debug for SitixFunction {
    fn fmt(&self, f : &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "<function>")
    }
}


impl std::cmp::PartialEq<Self> for SitixFunction {
    fn eq(&self, other : &SitixFunction) -> bool {
        false // TODO: make comparing functions a thing
    }
}


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
    Table(Vec<Data>,), // TODO: make tables what they're supposed to be (a HashMap<Data, Data> that also acts like a Vec indexed by Data::Numbers)
    Function(SitixFunction)
}

impl ToString for Data {
    fn to_string(&self) -> String {
        match self {
            Self::Boolean(b) => if *b { "true".to_string() } else { "false".to_string() },
            Self::Nil => "".to_string(),
            Self::Number(n) => n.to_string(),
            Self::String(s) => s.clone(),
            Self::Sitix(s) => s.clone(),
            Self::VariableHandle(u) => format!("variable handle {}", u),
            Self::Table(t) => format!("{:?}", t),
            Self::Function(f) => format!("<function>")
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

    pub fn force_function(&self) -> InterpretResult<SitixFunction> {
        if let Self::Function(data) = self {
            Ok(data.clone())
        }
        else {
            Err(InterpretError::InvalidType(format!("expected function, got {:?}", self)))
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

    pub fn load_standard_ffi(&mut self) {
        self.create_global("print".to_string(), Data::Function(SitixFunction::Builtin(&|interpreter, args : &[Data]| {
            for arg in args {
                print!("{}", arg.to_string());
            }
            println!("");
            Ok(Data::Nil)
        })));
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
            },
            Self::IfBranch(condition, truthy, falsey) => {
                let way = condition.interpret(i)?;
                let way = i.deref(way)?.force_boolean()?;
                if way {
                    truthy.interpret(i)
                }
                else if let Some(falsey) = falsey {
                    falsey.interpret(i)
                }
                else {
                    Ok(Data::Nil)
                }
            },
            Self::Table(table) => {
                let mut out = vec![];
                for expr in table {
                    out.push(expr.interpret(i)?);
                }
                Ok(Data::Table(out))
            },
            Self::While(cond, body) => {
                let mut out = String::new();
                loop {
                    let do_exec = cond.interpret(i)?;
                    let do_exec = i.deref(do_exec)?;
                    if do_exec.force_boolean()? {
                        let expressive_output = body.interpret(i)?;
                        let expressive_output = i.deref(expressive_output)?;
                        out += &expressive_output.to_string();
                    }
                    else {
                        break;
                    }
                }
                Ok(Data::String(out))
            },
            Self::Call(fun, args) => {
                let fun = fun.interpret(i)?;
                let fun = i.deref(fun)?;
                let mut to_args = vec![];
                for arg in args {
                    to_args.push(arg.interpret(i)?);
                }
                match fun.force_function()? {
                    SitixFunction::Builtin(built_in) => {
                        built_in(i, &to_args)
                    },
                    SitixFunction::UserDefined(req_args, contents) => {
                        i.open_scope();
                        if args.len() != req_args.len() {
                            panic!("invalid argument count (TODO: make this a real error)");
                        }
                        for (name, content) in req_args.into_iter().zip(to_args.into_iter()) {
                            let content = i.deref(content)?;
                            i.create(name, content);
                        }
                        let ret = contents.interpret(i);
                        i.close_scope();
                        ret
                    }
                }
            },
            Self::Function(args, contents) => {
                Ok(Data::Function(SitixFunction::UserDefined(args.clone(), contents.clone())))
            }
        }
    }
}

impl Unary {
    fn interpret(&self, i : &mut InterpreterState) -> InterpretResult<Data> {
        Ok(match self {
            Self::Negative(expr) => {
                let res = expr.interpret(i)?;
                let res = i.deref(res)?;
                Data::Number(res.force_number()? * -1.0)
            },
            Self::Not(expr) => {
                let res = expr.interpret(i)?;
                let res = i.deref(res)?;
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
                let one = i.deref(one)?.force_boolean()?;
                if one == false {
                    return Ok(Data::Boolean(false));
                }
                let two = two.interpret(i)?;
                let two = i.deref(two)?.force_boolean()?;
                Data::Boolean(one && two)
            },
            Self::Or(one, two) => {
                let one = one.interpret(i)?;
                let one = i.deref(one)?.force_boolean()?;
                if one == true {
                    return Ok(Data::Boolean(true));
                }
                let two = two.interpret(i)?;
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
            Self::Mod(one, two) => {
                let one = one.interpret(i)?;
                let two = two.interpret(i)?;
                let one = i.deref(one)?.force_number()?;
                let two = i.deref(two)?.force_number()?;
                Data::Number(one % two)
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
