// contains useful types
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Ident(String),
    String(String),
    Text(String),
    Number(f64),
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    BlockOpen, BlockClose(bool), // leftparen and rightparen respectively. the lexer actually has some context about sitix blocks, and passes them to the inflator because it's nice
                                 // if the bool in BlockClose is true, this block contains an expression
    LeftParen, RightParen, LeftBrace, RightBrace, LeftBracket, RightBracket,
    Comma, Dot, Plus, Minus, Slash, Star, Semicolon, Colon, At, DashTo,
    Eq, EqEq, Neq, Gt, Lt, Gte, Lte,
    PlusPlus, MinusMinus,
    PlusEq, MinusEq, StarEq, SlashEq,
    Literal(Literal),
    And, Or, Not,
    While, Each, If,
    True, False, Nil,
    Export
}

#[derive(Debug, Clone)]
pub struct Span {
    start_char : usize,
    end_char : usize
}

impl Span {
    pub fn new(start_char : usize, end_char : usize) -> Self {
        Self {
            start_char, end_char
        }
    }
}

#[derive(Debug, Clone)]
pub struct Token {
    pub tp : TokenType,
    pub span : Span
}


impl Token {
    pub fn new(tp : TokenType, span : Span) -> Self {
        Self {tp, span}
    }
}

impl std::fmt::Display for TokenType {
    fn fmt(&self, f : &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
