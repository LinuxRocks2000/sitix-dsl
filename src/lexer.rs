// contains the lexer function
use crate::utility::*;
use crate::error::{ SitixResult, Error };
use std::path::Path;


pub struct FileReader {
    name : String,
    span_start : usize,
    current_byte : usize,
    file : Vec<char> // todo: use a nice streaming solution for this
}


impl FileReader {
    pub fn open(name : impl AsRef<Path>) -> FileReader {
        FileReader {
            file : std::fs::read_to_string(&name).unwrap().chars().collect(),
            name : name.as_ref().file_name().unwrap().to_str().unwrap().to_string(),
            span_start : 0,
            current_byte : 0,
        }
    }

    fn skip_opening_phrase(&mut self) -> SitixResult<()> {
        self.next()?;
        self.next()?;
        self.next()?;
        Ok(())
    }

    fn next(&mut self) -> SitixResult<char> {
        self.current_byte += 1;
        self.file.get(self.current_byte - 1).ok_or(Error::unexpected_eof(self.top_span())).cloned()
    }

    fn peek(&self) -> SitixResult<char> {
        self.file.get(self.current_byte).ok_or(Error::unexpected_eof(self.top_span())).cloned()
    }

    fn top_span(&self) -> Span { // return the span of the single byte at the top of this reader
        Span::new(self.current_byte, self.current_byte, self.name.clone())
    }

    fn open_span(&mut self) {
        self.span_start = self.current_byte;
    }

    fn get_span(&self) -> Span {
        Span::new(self.span_start, self.current_byte, self.name.clone())
    }

    fn skip(&mut self, condition : impl Fn(char) -> bool) -> SitixResult<()> {
        while let Ok(c) = self.peek() {
            if condition(c) {
                self.next()?;
            }
            else {
                break;
            }
        }
        Ok(())
    }
}


pub fn lexer(mut buffer : FileReader) -> SitixResult<Vec<Token>> { // please for the love of god never touch this
                                                                   // [slightly later] it... it had to be touched. my... my eyes...
    buffer.skip_opening_phrase()?;
    let mut output = vec![];
    while let Ok(c) = buffer.peek() {
        buffer.open_span();
        match c {
            '[' => {
                buffer.next()?;
                output.push(Token::new(TokenType::BlockOpen, buffer.get_span()));
                let mut close_level = 1; // count open-brackets and close-brackets. if it reaches 0, we need to back into text buffering mode
                buffer.skip(char::is_whitespace)?;
                'inner_expression : while let Ok(c) = buffer.next() {
                    buffer.open_span();
                    if c.is_alphabetic() || c == '_' { // parse an ident
                        let mut idb = String::new();
                        idb.push(c);
                        while let Ok(c) = buffer.peek() {
                            if c.is_alphanumeric() || c == '_' {
                                idb.push(c);
                                buffer.next()?;
                            }
                            else {
                                break;
                            }
                        }
                        output.push(Token::new(match idb.as_str() {
                            "while" => TokenType::While,
                            "each" => TokenType::Each,
                            "if" => TokenType::If,
                            "else" => TokenType::Else,
                            "true" => TokenType::True,
                            "false" => TokenType::False,
                            "nil" => TokenType::Nil,
                            "and" => TokenType::And,
                            "or" => TokenType::Or,
                            "not" => TokenType::Not,
                            "let" => TokenType::Let,
                            "global" => TokenType::Global,
                            "debugger" => TokenType::Debugger,
                            "fun" => TokenType::Fun,
                            _ => { TokenType::Literal(Literal::Ident(idb)) }
                        }, buffer.get_span()));
                    }
                    else if let Some(c) = c.to_digit(10) {
                        let mut num_buf = c as f64;
                        while let Ok(c) = buffer.peek() {
                            if let Some(c) = c.to_digit(10) {
                                num_buf *= 10.0;
                                num_buf += c as f64;
                                buffer.next()?;
                            }
                            else if c == '.' {
                                buffer.next()?;
                                let mut pow = 0.1;
                                while let Ok(c) = buffer.peek() {
                                    if let Some(c) = c.to_digit(10) {
                                        num_buf += pow * c as f64;
                                        pow *= 0.1;
                                        buffer.next()?;
                                    }
                                    else {
                                        break;
                                    }
                                }
                            }
                            else {
                                break;
                            }
                        }
                        output.push(Token::new(TokenType::Literal(Literal::Number(num_buf)), buffer.get_span()));
                    }
                    else {
                        match c {
                            '"' => {
                                let mut str_buf = String::new();
                                while let Ok(c) = buffer.next() {
                                    if c == '\\' {
                                        str_buf.push(match buffer.next()? {
                                            'n' => '\n',
                                            'r' => '\r',
                                            '0' => '\0',
                                            c => c
                                        });
                                    }
                                    else if c == '"' {
                                        break;
                                    }
                                    else {
                                        str_buf.push(c);
                                    }
                                }
                                output.push(Token::new(TokenType::Literal(Literal::String(str_buf)), buffer.get_span()));
                            }
                            '[' => {
                                close_level += 1;
                                output.push(Token::new(TokenType::LeftBracket, buffer.get_span()));
                            }
                            ']' => {
                                close_level -= 1;
                                if close_level == 0 {
                                    output.push(Token::new(TokenType::BlockClose(false), buffer.get_span()));
                                    break 'inner_expression;
                                }
                                output.push(Token::new(TokenType::RightBracket, buffer.get_span()));
                            }
                            '{' => {
                                output.push(Token::new(TokenType::LeftBrace, buffer.get_span()));
                            }
                            '}' => {
                                output.push(Token::new(TokenType::RightBrace, buffer.get_span()));
                            }
                            '(' => {
                                output.push(Token::new(TokenType::LeftParen, buffer.get_span()));
                            }
                            ')' => {
                                output.push(Token::new(TokenType::RightParen, buffer.get_span()));
                            }
                            '.' => {
                                output.push(Token::new(TokenType::Dot, buffer.get_span()));
                            }
                            ',' => {
                                output.push(Token::new(TokenType::Comma, buffer.get_span()));
                            }
                            ';' => {
                                output.push(Token::new(TokenType::Semicolon, buffer.get_span()));
                            }
                            ':' => {
                                output.push(Token::new(TokenType::Colon, buffer.get_span()));
                            }
                            '@' => {
                                output.push(Token::new(TokenType::Fun, buffer.get_span()));
                            }
                            '+' => {
                                match buffer.peek()? {
                                    '=' => {
                                        buffer.next()?;
                                        output.push(Token::new(TokenType::PlusEq, buffer.get_span()));
                                    }
                                    '+' => {
                                        buffer.next()?;
                                        output.push(Token::new(TokenType::PlusPlus, buffer.get_span()));
                                    }
                                    _ => {
                                        output.push(Token::new(TokenType::Plus, buffer.get_span()));
                                    }
                                }
                            }
                            '-' => {
                                match buffer.peek()? {
                                    '=' => {
                                        buffer.next()?;
                                        output.push(Token::new(TokenType::MinusEq, buffer.get_span()));
                                    }
                                    '-' => {
                                        buffer.next()?;
                                        output.push(Token::new(TokenType::MinusMinus, buffer.get_span()));
                                    }
                                    '>' => {
                                        buffer.next()?;
                                        output.push(Token::new(TokenType::DashTo, buffer.get_span()));
                                    }
                                    ']' => {
                                        buffer.next()?;
                                        output.push(Token::new(TokenType::BlockClose(true), buffer.get_span()));
                                        break 'inner_expression;
                                    }
                                    _ => {
                                        output.push(Token::new(TokenType::Minus, buffer.get_span()));
                                    }
                                }
                            }
                            '%' => {
                                output.push(Token::new(TokenType::Modulo, buffer.get_span()));
                            }
                            '*' => {
                                if let Ok('=') = buffer.peek() {
                                    buffer.next()?;
                                    output.push(Token::new(TokenType::StarEq, buffer.get_span()));
                                }
                                else {
                                    output.push(Token::new(TokenType::Star, buffer.get_span()));
                                }
                            }
                            '/' => {
                                match buffer.peek()? {
                                    '=' => {
                                        buffer.next()?;
                                        output.push(Token::new(TokenType::SlashEq, buffer.get_span()));
                                    }
                                    '*' => {
                                        // it's a comment, skoob!
                                        buffer.next()?;
                                        'comment : while let Ok(c) = buffer.next() {
                                            if c == '*' {
                                                if let Ok('/') = buffer.peek() {
                                                    buffer.next()?;
                                                    break 'comment;
                                                }
                                            }
                                        }
                                    }
                                    _ => {
                                        output.push(Token::new(TokenType::Slash, buffer.get_span()));
                                    }
                                }
                            }
                            '=' => {
                                if buffer.peek()? == '=' {
                                    buffer.next()?;
                                    output.push(Token::new(TokenType::EqEq, buffer.get_span()));
                                }
                                else {
                                    output.push(Token::new(TokenType::Eq, buffer.get_span()));
                                }
                            }
                            '!' => {
                                if buffer.peek()? == '=' {
                                    buffer.next()?;
                                    output.push(Token::new(TokenType::Neq, buffer.get_span()));
                                }
                                else {
                                    output.push(Token::new(TokenType::Not, buffer.get_span()));
                                }
                            }
                            '>' => {
                                if buffer.peek()? == '=' {
                                    buffer.next()?;
                                    output.push(Token::new(TokenType::Gte, buffer.get_span()));
                                }
                                else {
                                    output.push(Token::new(TokenType::Gt, buffer.get_span()));
                                }
                            }
                            '<' => {
                                if buffer.peek()? == '=' {
                                    buffer.next()?;
                                    output.push(Token::new(TokenType::Lte, buffer.get_span()));
                                }
                                else {
                                    output.push(Token::new(TokenType::Lt, buffer.get_span()));
                                }
                            }
                            _ => { return Err(Error::unexpected_char(c, buffer.get_span())); }
                        }
                    }
                    buffer.skip(char::is_whitespace)?;
                }
            }
            _ => {
                let mut buf = String::new();
                while let Ok(c) = buffer.peek() {
                    if c == '\\' {
                        buffer.next()?;
                        if let Ok('[') = buffer.peek() {
                            buffer.next()?;
                            buf.push('[');
                        }
                        else {
                            buf.push('\\');
                        }
                    }
                    else if c == '[' {
                        break;
                    }
                    else {
                        buf.push(c);
                        buffer.next()?;
                    }
                }
                if buf.len() > 0 {
                    output.push(Token::new(TokenType::Literal(Literal::Text(buf)), buffer.get_span()));
                }
            }
        }
    }
    Ok(output)
}

