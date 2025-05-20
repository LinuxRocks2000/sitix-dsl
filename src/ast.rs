// see grammar.bnf

#[derive(Debug)]
pub enum SitixExpression {
    Block(Block),
    Text(String)
}

#[derive(Debug)]
pub enum Closing { // why have this? because eventually we'll want more complicated closing semantics
    Slash // the normal [/]
}

#[derive(Debug)]
pub struct Block {
    pub inner : Vec<Statement>,
    pub tail : Option<Statement>
}


#[derive(Debug)]
pub enum Statement {
    Expression(Box<Expression>), // a statement that does nothing but evaluate a tail-expression
    Print(Box<Expression>),
    LetAssign(String, Box<Expression>)
}


pub use crate::utility::Literal;

#[derive(Debug)]
pub enum Expression {
    Literal(Literal),
    Unary(Unary),
    Binary(Binary),
    Grouping(Box<Expression>),
    Braced(Box<Block>),
    SitixExpression(Vec<SitixExpression>), // the result of evaluating this is a complex object
    // that implicitly casts down to a String (equal to the concatenation of the stringifications
    // of every sitixexpression contained) and contains a number of properties (such as __filename__ for a file),
    // some of which are exports.
    True,
    False,
    Nil
}

#[derive(Debug)]
pub enum Unary {
    Negative(Box<Expression>),
    Not(Box<Expression>)
}

#[derive(Debug)]
pub enum Binary {
    Equals(Box<Expression>, Box<Expression>),
    Nequals(Box<Expression>, Box<Expression>),

    Add(Box<Expression>, Box<Expression>),
    Sub(Box<Expression>, Box<Expression>),
    Mul(Box<Expression>, Box<Expression>),
    Div(Box<Expression>, Box<Expression>),

    And(Box<Expression>, Box<Expression>),
    Or(Box<Expression>, Box<Expression>),

    Gt(Box<Expression>, Box<Expression>),
    Gte(Box<Expression>, Box<Expression>),
    Lt(Box<Expression>, Box<Expression>),
    Lte(Box<Expression>, Box<Expression>),
}


// 86.3, 12.5, 17.0
// -> city at -804.3, 2.5, 212.5
