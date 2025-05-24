// see grammar.bnf
use crate::utility::Span;


#[derive(Debug, Clone)]
pub enum SitixExpression {
    Block(Block),
    Text(String, Span)
}

#[derive(Debug, Clone)]
pub struct Block {
    pub inner : Vec<Statement>,
    pub tail : Option<Statement>,
    pub span : Span
}


#[derive(Debug, Clone)]
pub enum Statement {
    Expression(Box<Expression>), // a statement that does nothing but evaluate a tail-expression
    UnboundLetAssign(Span, String, Box<Expression>), // unbound: needs to be bound by resolver
    UnboundGlobalAssign(Span, String, Box<Expression>),
    Assign(Span, usize, Box<Expression>), // once bound, there's no useful distinction between `let` and `global`, so we only need one Assign
    Debugger(Span)
}


pub use crate::utility::Literal;

#[derive(Debug, Clone)]
pub enum Expression {
    Literal(Span, Literal),
    Unary(Unary),
    Binary(Binary),
    Grouping(Box<Expression>),
    Braced(Box<Block>),
    SitixExpression(Vec<SitixExpression>), // the result of evaluating this is a complex object
    // that implicitly casts down to a String (equal to the concatenation of the stringifications
    // of every sitixexpression contained) and contains a number of properties (such as __filename__ for a file),
    // some of which are exports.
    True(Span),
    False(Span),
    Nil(Span),
    UnboundVariableAccess(Span, String), // a variable access that has not been bound by the resolver
    VariableAccess(Span, usize), // a fully bound variable access
    Assignment(Box<Expression>, Box<Expression>),
    IfBranch(Span, Box<Expression>, Box<Expression>, Option<Box<Expression>>), // condition, true-branch, false-branch
    Table(Span, Vec<TableEntry>),
    While(Span, Box<Expression>, Box<Expression>),
    UnboundEach(Span, Box<Expression>, String, Option<String>, Box<Expression>),
    Each(Span, Box<Expression>, usize, Option<usize>, Box<Expression>),
    Call(Box<Expression>, Vec<Expression>),
    UnboundFunction(Span, Vec<(String, Span)>, Box<Expression>),
    Function(Span, Vec<(usize, Span)>, Box<Expression>)
}

#[derive(Debug, Clone)]
pub struct TableEntry {
    pub content : Box<Expression>,
    pub label : Option<String>
}

#[derive(Debug, Clone)]
pub enum Unary {
    Negative(Span, Box<Expression>),
    Not(Span, Box<Expression>)
}

#[derive(Debug, Clone)]
pub enum Binary {
    Equals(Box<Expression>, Box<Expression>),
    Nequals(Box<Expression>, Box<Expression>),

    Add(Box<Expression>, Box<Expression>),
    Sub(Box<Expression>, Box<Expression>),
    Mul(Box<Expression>, Box<Expression>),
    Div(Box<Expression>, Box<Expression>),
    Mod(Box<Expression>, Box<Expression>),

    And(Box<Expression>, Box<Expression>),
    Or(Box<Expression>, Box<Expression>),

    Gt(Box<Expression>, Box<Expression>),
    Gte(Box<Expression>, Box<Expression>),
    Lt(Box<Expression>, Box<Expression>),
    Lte(Box<Expression>, Box<Expression>),
}


// 86.3, 12.5, 17.0
// -> city at -804.3, 2.5, 212.5
