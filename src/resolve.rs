// variable binding resolver
use crate::ast::*;
use std::collections::HashMap;
use std::sync::Arc;
use crate::ffi::*;


pub struct ResolverState {
    scopes : Vec<HashMap<String, usize>>,
    top_var : usize,
    ffi : Arc<ForeignFunctionInterface>
}


impl ResolverState {
    pub fn new(ffi : Arc<ForeignFunctionInterface>) -> ResolverState {
        ResolverState {
            scopes : vec![HashMap::new()],
            top_var : ffi.top_index + 1,
            ffi
        }
    }

    fn open_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn close_scope(&mut self) {
        self.scopes.pop();
    }

    fn create(&mut self, name : String) -> usize {
        self.top_var += 1;
        self.scopes.last_mut().unwrap().insert(name, self.top_var);
        self.top_var
    }

    fn create_global(&mut self, name : String) -> usize {
        self.top_var += 1;
        self.scopes[0].insert(name, self.top_var);
        self.top_var
    }

    fn find(&mut self, name : &String) -> Option<usize> {
        for scope in self.scopes.iter().rev() {
            if let Some(ret) = scope.get(name) {
                return Some(*ret);
            }
        }
        self.ffi.find(name)
    }

    pub fn vomit(self) -> usize {
        self.top_var
    }

    pub fn settop(&mut self, top : usize) {
        self.top_var = top;
    }
}


impl SitixExpression {
    pub fn resolve(self, r : &mut ResolverState) -> Self {
        match self {
            Self::Block(b) => {
                r.open_scope();
                let ret = b.resolve(r);
                r.close_scope();
                return Self::Block(ret);
            },
            Self::Text(text, span) => Self::Text(text, span)
        }
    }
}

impl Block {
    fn resolve(mut self, r : &mut ResolverState) -> Self {
        r.open_scope();
        self.inner = self.inner.into_iter().map(|stmt| stmt.resolve(r)).collect();
        if let Some(tail) = self.tail {
            self.tail = Some(tail.resolve(r));
        }
        r.close_scope();
        self
    }
}

impl Statement {
    fn resolve(self, r : &mut ResolverState) -> Self {
        match self {
            Self::Expression(expr) => Self::Expression(Box::new(expr.resolve(r))),
            Self::UnboundLetAssign(tok, ident, expr) => {
                let id = r.create(ident);
                Self::Assign(tok, id, Box::new(expr.resolve(r)), None)
            },
            Self::UnboundGlobalAssign(tok, ident, expr) => {
                let id = r.create_global(ident.clone());
                Self::Assign(tok, id, Box::new(expr.resolve(r)), Some(ident))
            },
            _ => self
        }
    }
}

impl Expression {
    fn resolve(self, r : &mut ResolverState) -> Self {
        match self {
            Self::Literal(span, l) => Self::Literal(span, l.resolve(r)),
            Self::Unary(l) => Self::Unary(l.resolve(r)),
            Self::Binary(l) => Self::Binary(l.resolve(r)),
            Self::Grouping(l) => Self::Grouping(Box::new(l.resolve(r))),
            Self::Braced(l) => Self::Braced(Box::new(l.resolve(r))),
            Self::SitixExpression(l) => {
                Self::SitixExpression(l.into_iter().map(|expr| expr.resolve(r)).collect())
            },
            Self::True(s) => Self::True(s),
            Self::False(s) => Self::False(s),
            Self::Nil(s) => Self::Nil(s),
            Self::UnboundVariableAccess(span, name) => {
                Self::VariableAccess(span, r.find(&name).expect(&name)) // TODO: don't unwrap here
            },
            Self::Assignment(variable, value) => Self::Assignment(Box::new(variable.resolve(r)), Box::new(value.resolve(r))),
            Self::IfBranch(span, condition, truthy, falsey) => Self::IfBranch(span, Box::new(condition.resolve(r)), Box::new(truthy.resolve(r)), match falsey { Some(falsey) => Some(Box::new(falsey.resolve(r))), None => None }),
            Self::Table(span, table) => Self::Table(span, table.into_iter().map(|t| TableEntry { content : Box::new(t.content.resolve(r)), label : t.label }).collect()),
            Self::While(span, cond, body) => Self::While(span, Box::new(cond.resolve(r)), Box::new(body.resolve(r))),
            Self::Call(fun, args) => {
                let fun = fun.resolve(r);
                let args = args.into_iter().map(|arg| arg.resolve(r)).collect();
                Self::Call(Box::new(fun), args)
            },
            Self::Function(span, args, contents) => Self::Function(span, args, Box::new(contents.resolve(r))),
            Self::VariableAccess(span, v) => Self::VariableAccess(span, v),
            Self::UnboundFunction(span, args, cont) => {
                r.open_scope();
                let args = args.into_iter().map(|(arg, span)| (r.create(arg), span)).collect();
                let cont = cont.resolve(r);
                r.close_scope();
                Self::Function(span, args, Box::new(cont))
            },
            Self::UnboundEach(span, cond, var, secondary_var, cont) => {
                let cond = cond.resolve(r);
                r.open_scope();
                let var = r.create(var);
                let autrevar = if let Some(s) = secondary_var {
                    Some(r.create(s))
                } else { None };
                let cont = cont.resolve(r);
                r.close_scope();
                if let Some(autrevar) = autrevar {
                    Self::Each(span, Box::new(cond), autrevar, Some(var), Box::new(cont))
                }
                else {
                    Self::Each(span, Box::new(cond), var, None, Box::new(cont))
                }
            },
            Self::Each(_, _, _, _, _) => panic!("unreachable"),
            Self::DotAccess(expr, s) => Self::DotAccess(Box::new(expr.resolve(r)), s)
        }
    }
}

impl Unary {
    fn resolve(self, r : &mut ResolverState) -> Self {
        match self {
            Self::Negative(span, expr) => {
                Self::Negative(span, Box::new(expr.resolve(r)))
            },
            Self::Not(span, expr) => {
                Self::Not(span, Box::new(expr.resolve(r)))
            }
        }
    }
}

impl Literal {
    fn resolve(self, _ : &mut ResolverState) -> Self {
        self
    }
}

impl Binary {
    fn resolve(self, r : &mut ResolverState) -> Self {
        match self {
            Self::Equals(one, two) => Self::Equals(Box::new(one.resolve(r)), Box::new(two.resolve(r))),
            Self::Nequals(one, two) => Self::Nequals(Box::new(one.resolve(r)), Box::new(two.resolve(r))),
            Self::And(one, two) => Self::And(Box::new(one.resolve(r)), Box::new(two.resolve(r))),
            Self::Or(one, two) => Self::Or(Box::new(one.resolve(r)), Box::new(two.resolve(r))),
            Self::Add(one, two) => Self::Add(Box::new(one.resolve(r)), Box::new(two.resolve(r))),
            Self::Sub(one, two) => Self::Sub(Box::new(one.resolve(r)), Box::new(two.resolve(r))),
            Self::Mul(one, two) => Self::Mul(Box::new(one.resolve(r)), Box::new(two.resolve(r))),
            Self::Div(one, two) => Self::Div(Box::new(one.resolve(r)), Box::new(two.resolve(r))),
            Self::Mod(one, two) => Self::Mod(Box::new(one.resolve(r)), Box::new(two.resolve(r))),
            Self::Gt(one, two) => Self::Gt(Box::new(one.resolve(r)), Box::new(two.resolve(r))),
            Self::Gte(one, two) => Self::Gte(Box::new(one.resolve(r)), Box::new(two.resolve(r))),
            Self::Lt(one, two) => Self::Lt(Box::new(one.resolve(r)), Box::new(two.resolve(r))),
            Self::Lte(one, two) => Self::Lte(Box::new(one.resolve(r)), Box::new(two.resolve(r)))
        }
    }
}
