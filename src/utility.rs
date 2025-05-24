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
    Comma, Dot, Plus, Minus, Slash, Star, Modulo, Semicolon, Colon, DashTo,
    Eq, EqEq, Neq, Gt, Lt, Gte, Lte,
    PlusPlus, MinusMinus,
    PlusEq, MinusEq, StarEq, SlashEq,
    Literal(Literal),
    And, Or, Not,
    While, Each, If, Else,
    True, False, Nil,
    Let, Global, Fun,
    Debugger
}

#[derive(Debug, Clone)]
pub struct Span {
    pub start_char : usize,
    pub end_char : usize,
    pub filename : String
}

impl Span {
    pub fn new(start_char : usize, end_char : usize, filename : String) -> Self {
        Self {
            start_char, end_char, filename
        }
    }

    pub fn identity() -> Self {
        Self::new(0, 0, "unknown_file".to_string())
    }

    pub fn get_line_col(&self) -> (usize, usize) {
        let mut line = 1;
        let mut col = 0;
        let file = match std::fs::read_to_string(&self.filename) { Ok(file) => file, Err(_) => {return (0, 0);}};
        let iter = file.chars();
        let mut cnt = self.start_char;
        for ch in iter {
            cnt -= 1;
            if ch == '\n' {
                col = 0;
                line += 1;
            }
            if cnt == 0 {
                break;
            }
            col += 1;
        }
        (line, col)
    }

    pub fn merge(self, other : Span) -> Span {
        Span {
            filename : self.filename,
            start_char : self.start_char.min(other.start_char),
            end_char : self.end_char.max(other.end_char)
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
