// impls for all of the ast types that give them the interpret() function

use crate::ast::*;
use std::collections::HashMap;
use std::sync::Arc;
use crate::ffi::*;
use crate::error::*;
use crate::utility::Span;


#[derive(Clone)]
pub enum SitixFunction {
    Builtin(&'static dyn Fn(&mut InterpreterState, &[Data]) -> SitixResult<Data>),
    UserDefined(Vec<(usize, Span)>, Box<Expression>)
}


impl std::fmt::Debug for SitixFunction {
    fn fmt(&self, f : &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "<function>")
    }
}


impl std::cmp::PartialEq<Self> for SitixFunction {
    fn eq(&self, _ : &SitixFunction) -> bool {
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
    Table(HashMap<usize, Data>, HashMap<String, Data>), // Is This Stupid, Or What?
                                             // essentially: can't easily hash f64, don't want to
                                             // use a Key enum with downcasts from f64 to u64,
                                             // hence: tables are two hashmaps, one from usize and one
                                             // from String. Dumb 'Nuff 4 Ya?
                                             // and yes, floats *are* rounded for indexing.
                                             // heh.
                                             // heh heh.
                                             // HAHAHAHAHAHAHAHAHAHAHA.
                                             // *ahem*, sorry, just remembering that time I philosophized
                                             // about making a language better than JavaScript
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
            Self::Table(t, p) => format!("{:?} + {:?}", t, p),
            Self::Function(_) => format!("<function>")
        }
    }
}


impl Data {
    pub fn force_boolean(&self) -> SitixPartialResult<bool> {
        if let Self::Boolean(data) = self {
            Ok(*data)
        }
        else {
            Err(PartialError::invalid_type("boolean", self.typename()))
        }
    }

    pub fn force_number(&self) -> SitixPartialResult<f64> {
        if let Self::Number(data) = self {
            Ok(*data)
        }
        else {
            Err(PartialError::invalid_type("number", self.typename()))
        }
    }

    pub fn force_function(&self) -> SitixPartialResult<SitixFunction> {
        if let Self::Function(data) = self {
            Ok(data.clone())
        }
        else {
            Err(PartialError::invalid_type("function", self.typename()))
        }
    }

    pub fn force_table(self) -> SitixPartialResult<(HashMap<usize, Data>, HashMap<String, Data>)> {
        if let Self::Table(int_data, string_data) = self {
            Ok((int_data, string_data))
        }
        else {
            Err(PartialError::invalid_type("function", self.typename()))
        }
    }

    pub fn typename(&self) -> String {
        match self {
            Self::Boolean(_) => "boolean",
            Self::Nil => "niltype",
            Self::Number(_) => "number",
            Self::String(_) => "string",
            Self::Sitix(_) => "text",
            Self::VariableHandle(_) => "reference",
            Self::Table(_, _) => "table",
            Self::Function(_) => "function"
        }.to_string()
    }
}


#[derive(Debug)]
pub struct InterpreterState {
    variables : HashMap<usize, Data>,
    ffi : Arc<ForeignFunctionInterface>
}


impl InterpreterState {
    pub fn new(ffi : Arc<ForeignFunctionInterface>) -> Self {
        Self {
            variables : HashMap::new(),
            ffi
        }
    }

    pub fn get(&self, index : usize) -> SitixPartialResult<Data> {
        if let Some(_) = self.variables.get(&index) {
            return Ok(Data::VariableHandle(index));
        }
        else if let Some(_) = self.ffi.get(index) {
            return Ok(Data::VariableHandle(index));
        }
        Err(PartialError::undefined_symbol())
    }

    pub fn create(&mut self, ident : usize, data : Data) -> Data {
        self.variables.insert(ident, data);
        Data::VariableHandle(ident)
    }

    pub fn set(&mut self, handle : Data, data : Data) -> SitixPartialResult<()> {
        if let Data::VariableHandle(u) = handle {
            if let Some(var) = self.variables.get_mut(&u) {
                *var = data;
                Ok(())
            }
            else {
                Err(PartialError::undefined_symbol())
            }
        }
        else {
            Err(PartialError::invalid_type("variable", handle.typename()))
        }
    }

    pub fn deref(&self, data : Data) -> SitixPartialResult<Data> {
        match data {
            Data::VariableHandle(index) => {
                if let Some(var) = self.variables.get(&index).cloned() {
                    Ok(var)
                }
                else if let Some(var) = self.ffi.get(index) {
                    Ok(var)
                }
                else {
                    Err(PartialError::undefined_symbol())
                }
            },
            _ => Ok(data)
        }
    }
}


impl SitixExpression {
    pub fn interpret(&self, interpreter : &mut InterpreterState) -> SitixResult<Data> {
        Ok(match self {
            Self::Block(b) => {
                Data::Sitix(b.interpret(interpreter)?.to_string())
            },
            Self::Text(text, _) => Data::Sitix(text.clone())
        })
    }

    pub fn blame(&self) -> Span { // returns the whole span of a given subtree
        match self {
            Self::Block(b) => b.blame(),
            Self::Text(_, s) => s.clone()
        }
    }
}

impl Block {
    fn interpret(&self, i : &mut InterpreterState) -> SitixResult<Data> {
        for statement in &self.inner {
            statement.interpret(i)?; // throw away the result
        }
        if let Some(tail) = &self.tail {
            let out = tail.interpret(i)?;
            let out = i.deref(out).map_err(|e| e.weld(tail.blame()))?;
            Ok(out)
        }
        else {
            Ok(Data::Nil)
        }
    }

    pub fn blame(&self) -> Span {
        self.span.clone()
    }
}

impl Statement {
    fn interpret(&self, i : &mut InterpreterState) -> SitixResult<Data> {
        match self {
            Self::Expression(expr) => expr.interpret(i),
            Self::Assign(_, ident, expr) => {
                let value = expr.interpret(i)?;
                i.create(*ident, value);
                Ok(Data::Nil)
            },
            Self::Debugger(_) => {
                println!("==DEBUGGER==\nstate is {:?}", i);
                Ok(Data::Nil)
            },
            _ => panic!("unreachable: did you resolve() the syntax tree?")
        }
    }

    fn blame(&self) -> Span {
        match self {
            Self::Expression(expr) => expr.blame(),
            Self::UnboundLetAssign(_, _, _) => panic!("unreachable"),
            Self::UnboundGlobalAssign(_, _, _) => panic!("unreachable"),
            Self::Assign(span, _, expr) => {
                span.clone().merge(expr.blame())
            },
            Self::Debugger(span) => {
                span.clone()
            }
        }
    }
}

impl Expression {
    fn interpret(&self, i : &mut InterpreterState) -> SitixResult<Data> {
        match self {
            Self::Literal(_, l) => l.interpret(i),
            Self::Unary(u) => u.interpret(i),
            Self::Binary(b) => b.interpret(i),
            Self::Grouping(e) => e.interpret(i),
            Self::Braced(b) => b.interpret(i),
            Self::SitixExpression(v) => {
                let mut result = String::new();
                for expr in v {
                    let r = expr.interpret(i)?;
                    result += &i.deref(r).map_err(|e| e.weld(expr.blame()))?.to_string();
                }
                Ok(Data::Sitix(result))
            },
            Self::True(_) => Ok(Data::Boolean(true)),
            Self::False(_) => Ok(Data::Boolean(false)),
            Self::Nil(_) => Ok(Data::Nil),
            Self::VariableAccess(span, name) => {
                i.get(*name).map_err(|e| e.weld(span.clone()))
            },
            Self::Assignment(variable, value) => {
                let var = variable.interpret(i)?;
                let val = value.interpret(i)?;
                i.set(var, i.deref(val.clone()).map_err(|e| e.weld(value.blame()))?).map_err(|e| e.weld(variable.blame()))?;
                Ok(val)
            },
            Self::IfBranch(_, condition, truthy, falsey) => {
                let way = condition.interpret(i)?;
                let way = i.deref(way).map_err(|e| e.weld(condition.blame()))?.force_boolean().map_err(|e| e.weld(condition.blame()))?;
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
            Self::Table(_, table) => {
                let mut int_out = HashMap::new();
                let mut string_out = HashMap::new();
                let mut current_index = 0;
                for entry in table {
                    let expr = entry.content.interpret(i)?;
                    let expr = i.deref(expr).map_err(|e| e.weld(entry.content.blame()))?;
                    if let Some(label) = &entry.label {
                        string_out.insert(label.clone(), expr);
                    }
                    else {
                        int_out.insert(current_index, expr);
                        current_index += 1;
                    }
                }
                Ok(Data::Table(int_out, string_out))
            },
            Self::While(_, cond, body) => {
                let mut out = String::new();
                loop {
                    let do_exec = cond.interpret(i)?;
                    let do_exec = i.deref(do_exec).map_err(|e| e.weld(cond.blame()))?;
                    if do_exec.force_boolean().map_err(|e| e.weld(cond.blame()))? {
                        let expressive_output = body.interpret(i)?;
                        let expressive_output = i.deref(expressive_output).map_err(|e| e.weld(body.blame()))?;
                        out += &expressive_output.to_string();
                    }
                    else {
                        break;
                    }
                }
                Ok(Data::String(out))
            },
            Self::Call(func, args) => {
                let fun = func.interpret(i)?;
                let fun = i.deref(fun).map_err(|e| e.weld(func.blame()))?;
                let mut to_args = vec![];
                for arg in args {
                    to_args.push(arg.interpret(i)?);
                }
                match fun.force_function().map_err(|e| e.weld(func.blame()))? {
                    SitixFunction::Builtin(built_in) => {
                        built_in(i, &to_args)
                    },
                    SitixFunction::UserDefined(req_args, contents) => {
                        if args.len() != req_args.len() {
                            panic!("invalid argument count (TODO: make this a real error)");
                        }
                        for ((id, span), content) in req_args.into_iter().zip(to_args.into_iter()) {
                            let content = i.deref(content).map_err(|e| e.weld(span.clone()))?;
                            i.create(id, content);
                        }
                        let ret = contents.interpret(i);
                        ret
                    }
                }
            },
            Self::Function(_, args, contents) => {
                Ok(Data::Function(SitixFunction::UserDefined(args.clone(), contents.clone())))
            },
            Self::Each(span, cond, var, second_var, body) => {
                let mut out = String::new();
                let array = cond.interpret(i)?;
                let array = i.deref(array).map_err(|e| e.weld(span.clone()))?;
                let (int_map, string_map) = array.force_table().map_err(|e| e.weld(span.clone()))?;
                for (index, item) in &int_map {
                    i.create(*var, item.clone());
                    if let Some(v) = second_var {
                        i.create(*v, Data::Number(*index as f64));
                    }
                    let expr_out = body.interpret(i)?;
                    let expr_out = i.deref(expr_out).map_err(|e| e.weld(body.blame()))?;
                    out += &expr_out.to_string();
                }
                for (index, item) in &string_map {
                    i.create(*var, item.clone());
                    if let Some(v) = second_var {
                        i.create(*v, Data::String(index.clone()));
                    }
                    let expr_out = body.interpret(i)?;
                    let expr_out = i.deref(expr_out).map_err(|e| e.weld(body.blame()))?;
                    out += &expr_out.to_string();
                }
                Ok(Data::String(out))
            }
            _ => panic!("unreachable")
        }
    }

    fn blame(&self) -> Span {
        match self {
            Self::Literal(span, _) => span.clone(),
            Self::Unary(u) => u.blame(),
            Self::Binary(b) => b.blame(),
            Self::Grouping(e) => e.blame(),
            Self::Braced(b) => b.blame(),
            Self::SitixExpression(v) => {
                let mut start = v[0].blame();
                for expr in &v[1..] {
                    start = start.merge(expr.blame());
                }
                start
            },
            Self::True(span) => span.clone(),
            Self::False(span) => span.clone(),
            Self::Nil(span) => span.clone(),
            Self::VariableAccess(span, _) => span.clone(),
            Self::Assignment(variable, value) => variable.blame().merge(value.blame()),
            Self::IfBranch(span, _, truthy, _) => span.clone().merge(truthy.blame()),
            Self::Table(span, _) => span.clone(),
            Self::While(span, _, body) => span.clone().merge(body.blame()),
            Self::Call(fun, args) => if let Some(last) = args.last() { fun.blame().merge(last.blame()) } else { fun.blame() },
            Self::Function(span, _, contents) => span.clone().merge(contents.blame()),
            _ => panic!("unreachable")
        }
    }
}

impl Unary {
    fn interpret(&self, i : &mut InterpreterState) -> SitixResult<Data> {
        Ok(match self {
            Self::Negative(span, expr) => {
                let res = expr.interpret(i)?;
                let res = i.deref(res).map_err(|e| e.weld(span.clone().merge(expr.blame())))?;
                Data::Number(res.force_number().map_err(|e| e.weld(span.clone().merge(expr.blame())))? * -1.0)
            },
            Self::Not(span, expr) => {
                let res = expr.interpret(i)?;
                let res = i.deref(res).map_err(|e| e.weld(span.clone().merge(expr.blame())))?;
                Data::Boolean(!(res.force_boolean().map_err(|e| e.weld(span.clone().merge(expr.blame())))?))
            }
        })
    }

    fn blame(&self) -> Span {
        match self {
            Self::Negative(span, expr) => span.clone().merge(expr.blame()),
            Self::Not(span, expr) => span.clone().merge(expr.blame()),
        }
    }
}

impl Literal {
    fn interpret(&self, _ : &mut InterpreterState) -> SitixResult<Data> {
        Ok(match self {
            Self::Ident(_) => todo!("identifier lookup"),
            Self::String(s) => Data::String(s.clone()),
            Self::Text(s) => Data::Sitix(s.clone()),
            Self::Number(n) => Data::Number(*n)
        })
    }
}

impl Binary {
    fn interpret(&self, i : &mut InterpreterState) -> SitixResult<Data> {
        Ok(match self {
            Self::Equals(_one, _two) => {
                let one = _one.interpret(i)?;
                let two = _two.interpret(i)?;
                let one = i.deref(one).map_err(|e| e.weld(_one.blame().merge(_two.blame())))?;
                let two = i.deref(two).map_err(|e| e.weld(_one.blame().merge(_two.blame())))?;
                Data::Boolean(one == two)
            },
            Self::Nequals(_one, _two) => {
                let one = _one.interpret(i)?;
                let two = _two.interpret(i)?;
                let one = i.deref(one).map_err(|e| e.weld(_one.blame().merge(_two.blame())))?;
                let two = i.deref(two).map_err(|e| e.weld(_one.blame().merge(_two.blame())))?;
                Data::Boolean(one != two)
            },
            Self::And(_one, _two) => {
                let one = _one.interpret(i)?;
                let one = i.deref(one).map_err(|e| e.weld(_one.blame().merge(_two.blame())))?
                    .force_boolean().map_err(|e| e.weld(_one.blame().merge(_two.blame())))?;
                if one == false {
                    return Ok(Data::Boolean(false));
                }
                let two = _two.interpret(i)?;
                let two = i.deref(two).map_err(|e| e.weld(_one.blame().merge(_two.blame())))?
                    .force_boolean().map_err(|e| e.weld(_one.blame().merge(_two.blame())))?;
                Data::Boolean(one && two)
            },
            Self::Or(_one, _two) => {
                let one = _one.interpret(i)?;
                let one = i.deref(one).map_err(|e| e.weld(_one.blame().merge(_two.blame())))?
                    .force_boolean().map_err(|e| e.weld(_one.blame().merge(_two.blame())))?;
                if one == true {
                    return Ok(Data::Boolean(true));
                }
                let two = _two.interpret(i)?;
                let two = i.deref(two).map_err(|e| e.weld(_one.blame().merge(_two.blame())))?
                    .force_boolean().map_err(|e| e.weld(_one.blame().merge(_two.blame())))?;
                Data::Boolean(one || two)
            },
            Self::Add(_one, _two) => {
                let one = _one.interpret(i)?;
                let two = _two.interpret(i)?;
                let one = i.deref(one).map_err(|e| e.weld(_one.blame().merge(_two.blame())))?;
                let two = i.deref(two).map_err(|e| e.weld(_one.blame().merge(_two.blame())))?;
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
                    let one = one.force_number().map_err(|e| e.weld(_one.blame().merge(_two.blame())))?;
                    let two = two.force_number().map_err(|e| e.weld(_one.blame().merge(_two.blame())))?;
                    Data::Number(one + two)
                }
            },
            Self::Sub(_one, _two) => {
                let one = _one.interpret(i)?;
                let two = _two.interpret(i)?;
                let one = i.deref(one).map_err(|e| e.weld(_one.blame().merge(_two.blame())))?
                    .force_number().map_err(|e| e.weld(_one.blame().merge(_two.blame())))?;
                let two = i.deref(two).map_err(|e| e.weld(_one.blame().merge(_two.blame())))?
                    .force_number().map_err(|e| e.weld(_one.blame().merge(_two.blame())))?;
                Data::Number(one - two)
            },
            Self::Mul(_one, _two) => {
                let one = _one.interpret(i)?;
                let two = _two.interpret(i)?;
                let one = i.deref(one).map_err(|e| e.weld(_one.blame().merge(_two.blame())))?
                    .force_number().map_err(|e| e.weld(_one.blame().merge(_two.blame())))?;
                let two = i.deref(two).map_err(|e| e.weld(_one.blame().merge(_two.blame())))?
                    .force_number().map_err(|e| e.weld(_one.blame().merge(_two.blame())))?;
                Data::Number(one * two)
            },
            Self::Div(_one, _two) => {
                let one = _one.interpret(i)?;
                let two = _two.interpret(i)?;
                let one = i.deref(one).map_err(|e| e.weld(_one.blame().merge(_two.blame())))?
                    .force_number().map_err(|e| e.weld(_one.blame().merge(_two.blame())))?;
                let two = i.deref(two).map_err(|e| e.weld(_one.blame().merge(_two.blame())))?
                    .force_number().map_err(|e| e.weld(_one.blame().merge(_two.blame())))?;
                Data::Number(one / two)
            },
            Self::Mod(_one, _two) => {
                let one = _one.interpret(i)?;
                let two = _two.interpret(i)?;
                let one = i.deref(one).map_err(|e| e.weld(_one.blame().merge(_two.blame())))?
                    .force_number().map_err(|e| e.weld(_one.blame().merge(_two.blame())))?;
                let two = i.deref(two).map_err(|e| e.weld(_one.blame().merge(_two.blame())))?
                    .force_number().map_err(|e| e.weld(_one.blame().merge(_two.blame())))?;
                Data::Number(one % two)
            },
            Self::Gt(_one, _two) => {
                let one = _one.interpret(i)?;
                let two = _two.interpret(i)?;
                let one = i.deref(one).map_err(|e| e.weld(_one.blame().merge(_two.blame())))?
                    .force_number().map_err(|e| e.weld(_one.blame().merge(_two.blame())))?;
                let two = i.deref(two).map_err(|e| e.weld(_one.blame().merge(_two.blame())))?
                    .force_number().map_err(|e| e.weld(_one.blame().merge(_two.blame())))?;
                Data::Boolean(one > two)
            },
            Self::Gte(_one, _two) => {
                let one = _one.interpret(i)?;
                let two = _two.interpret(i)?;
                let one = i.deref(one).map_err(|e| e.weld(_one.blame().merge(_two.blame())))?
                    .force_number().map_err(|e| e.weld(_one.blame().merge(_two.blame())))?;
                let two = i.deref(two).map_err(|e| e.weld(_one.blame().merge(_two.blame())))?
                    .force_number().map_err(|e| e.weld(_one.blame().merge(_two.blame())))?;
                Data::Boolean(one >= two)
            },
            Self::Lt(_one, _two) => {
                let one = _one.interpret(i)?;
                let two = _two.interpret(i)?;
                let one = i.deref(one).map_err(|e| e.weld(_one.blame().merge(_two.blame())))?
                    .force_number().map_err(|e| e.weld(_one.blame().merge(_two.blame())))?;
                let two = i.deref(two).map_err(|e| e.weld(_one.blame().merge(_two.blame())))?
                    .force_number().map_err(|e| e.weld(_one.blame().merge(_two.blame())))?;
                Data::Boolean(one < two)
            },
            Self::Lte(_one, _two) => {
                let one = _one.interpret(i)?;
                let two = _two.interpret(i)?;
                let one = i.deref(one).map_err(|e| e.weld(_one.blame().merge(_two.blame())))?
                    .force_number().map_err(|e| e.weld(_one.blame().merge(_two.blame())))?;
                let two = i.deref(two).map_err(|e| e.weld(_one.blame().merge(_two.blame())))?
                    .force_number().map_err(|e| e.weld(_one.blame().merge(_two.blame())))?;
                Data::Boolean(one <= two)
            }
        })
    }

    fn blame(&self) -> Span {
        match self {
            Self::Equals(one, two) => one.blame().merge(two.blame()),
            Self::Nequals(one, two) => one.blame().merge(two.blame()),
            Self::And(one, two) => one.blame().merge(two.blame()),
            Self::Or(one, two) => one.blame().merge(two.blame()),
            Self::Add(one, two) => one.blame().merge(two.blame()),
            Self::Sub(one, two) => one.blame().merge(two.blame()),
            Self::Mul(one, two) => one.blame().merge(two.blame()),
            Self::Div(one, two) => one.blame().merge(two.blame()),
            Self::Mod(one, two) => one.blame().merge(two.blame()),
            Self::Gt(one, two) => one.blame().merge(two.blame()),
            Self::Gte(one, two) => one.blame().merge(two.blame()),
            Self::Lt(one, two) => one.blame().merge(two.blame()),
            Self::Lte(one, two) => one.blame().merge(two.blame())
        }
    }
}
