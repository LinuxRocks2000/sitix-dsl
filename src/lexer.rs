// contains the lexer function
use crate::lookahead::*;
use crate::utility::*;


pub fn lexer(mut buffer : impl LookaheadBuffer<char>) -> Vec<Token> { // please for the love of god never touch this
    let mut output = vec![];
    while let Some(c) = buffer.peek() {
        let span_start = buffer.get_head();
        match c {
            '[' => {
                buffer.next();
                output.push(Token::new(TokenType::BlockOpen, Span::new(span_start, span_start)));
                let mut close_level = 1; // count open-brackets and close-brackets. if it reaches 0, we need to back into text buffering mode
                buffer.skip(char::is_whitespace);
                'inner_expression : while let Some(c) = buffer.next() {
                    let span_start = buffer.get_head();
                    if c.is_alphabetic() { // parse an ident
                        let ident_start = buffer.get_head();
                        let mut idb = String::new();
                        idb.push(c);
                        while let Some(c) = buffer.peek() {
                            if c.is_alphanumeric() || c == '_' {
                                idb.push(c);
                                buffer.next();
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
                        }, Span::new(ident_start, buffer.get_head())));
                    }
                    else if c.is_numeric() {
                        let mut num_buf = c.to_digit(10).unwrap() as f64;
                        while let Some(c) = buffer.peek() {
                            if c.is_numeric() {
                                num_buf *= 10.0;
                                num_buf += c.to_digit(10).unwrap() as f64;
                                buffer.next();
                            }
                            else if c == '.' {
                                buffer.next();
                                let mut pow = 0.1;
                                while let Some(c) = buffer.peek() {
                                    if c.is_numeric() {
                                        num_buf += pow * c.to_digit(10).unwrap() as f64;
                                        pow *= 0.1;
                                        buffer.next();
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
                        output.push(Token::new(TokenType::Literal(Literal::Number(num_buf)), Span::new(span_start, buffer.get_head())));
                    }
                    else {
                        match c {
                            '"' => {
                                let mut str_buf = String::new();
                                while let Some(c) = buffer.next() {
                                    if c == '\\' {
                                        str_buf.push(match buffer.next() {
                                            Some('n') => '\n',
                                            Some('r') => '\r',
                                            Some('0') => '\0',
                                            Some(c) => c,
                                            None => { panic!("unexpected EOF"); }
                                        });
                                    }
                                    else if c == '"' {
                                        break;
                                    }
                                    else {
                                        str_buf.push(c);
                                    }
                                }
                                output.push(Token::new(TokenType::Literal(Literal::String(str_buf)), Span::new(span_start, buffer.get_head())));
                            }
                            '[' => {
                                close_level += 1;
                                output.push(Token::new(TokenType::LeftBracket, Span::new(span_start, span_start)));
                            }
                            ']' => {
                                close_level -= 1;
                                if close_level == 0 {
                                    output.push(Token::new(TokenType::BlockClose(false), Span::new(span_start, span_start)));
                                    break 'inner_expression;
                                }
                                output.push(Token::new(TokenType::RightBracket, Span::new(span_start, span_start)));
                            }
                            '{' => {
                                output.push(Token::new(TokenType::LeftBrace, Span::new(span_start, span_start)));
                            }
                            '}' => {
                                output.push(Token::new(TokenType::RightBrace, Span::new(span_start, span_start)));
                            }
                            '(' => {
                                output.push(Token::new(TokenType::LeftParen, Span::new(span_start, span_start)));
                            }
                            ')' => {
                                output.push(Token::new(TokenType::RightParen, Span::new(span_start, span_start)));
                            }
                            '.' => {
                                output.push(Token::new(TokenType::Dot, Span::new(span_start, span_start)));
                            }
                            ',' => {
                                output.push(Token::new(TokenType::Comma, Span::new(span_start, span_start)));
                            }
                            ';' => {
                                output.push(Token::new(TokenType::Semicolon, Span::new(span_start, span_start)));
                            }
                            ':' => {
                                output.push(Token::new(TokenType::Colon, Span::new(span_start, span_start)));
                            }
                            '@' => {
                                output.push(Token::new(TokenType::Fun, Span::new(span_start, span_start)));
                            }
                            '+' => {
                                match buffer.peek() {
                                    Some('=') => {
                                        buffer.next();
                                        output.push(Token::new(TokenType::PlusEq, Span::new(span_start, buffer.get_head())));
                                    }
                                    Some('+') => {
                                        buffer.next();
                                        output.push(Token::new(TokenType::PlusPlus, Span::new(span_start, buffer.get_head())));
                                    }
                                    _ => {
                                        output.push(Token::new(TokenType::Plus, Span::new(span_start, span_start)));
                                    }
                                }
                            }
                            '-' => {
                                match buffer.peek() {
                                    Some('=') => {
                                        buffer.next();
                                        output.push(Token::new(TokenType::MinusEq, Span::new(span_start, buffer.get_head())));
                                    }
                                    Some('-') => {
                                        buffer.next();
                                        output.push(Token::new(TokenType::MinusMinus, Span::new(span_start, buffer.get_head())));
                                    }
                                    Some('>') => {
                                        buffer.next();
                                        output.push(Token::new(TokenType::DashTo, Span::new(span_start, buffer.get_head())));
                                    }
                                    Some(']') => {
                                        buffer.next();
                                        output.push(Token::new(TokenType::BlockClose(true), Span::new(span_start, buffer.get_head())));
                                        break 'inner_expression;
                                    }
                                    _ => {
                                        output.push(Token::new(TokenType::Minus, Span::new(span_start, span_start)));
                                    }
                                }
                            }
                            '%' => {
                                output.push(Token::new(TokenType::Modulo, Span::new(span_start, buffer.get_head())));
                            }
                            '*' => {
                                if let Some('=') = buffer.peek() {
                                    buffer.next();
                                    output.push(Token::new(TokenType::StarEq, Span::new(span_start, buffer.get_head())));
                                }
                                else {
                                    output.push(Token::new(TokenType::Star, Span::new(span_start, span_start)));
                                }
                            }
                            '/' => {
                                match buffer.peek() {
                                    Some('=') => {
                                        buffer.next();
                                        output.push(Token::new(TokenType::SlashEq, Span::new(span_start, buffer.get_head())));
                                    }
                                    Some('*') => {
                                        // it's a comment, skoob!
                                        buffer.next();
                                        'comment : while let Some(c) = buffer.next() {
                                            if c == '*' {
                                                if let Some('/') = buffer.peek() {
                                                    buffer.next();
                                                    break 'comment;
                                                }
                                            }
                                        }
                                    }
                                    _ => {
                                        output.push(Token::new(TokenType::Slash, Span::new(span_start, span_start)));
                                    }
                                }
                            }
                            '=' => {
                                if let Some('=') = buffer.peek() {
                                    buffer.next();
                                    output.push(Token::new(TokenType::EqEq, Span::new(span_start, buffer.get_head())));
                                }
                                else {
                                    output.push(Token::new(TokenType::Eq, Span::new(span_start, span_start)));
                                }
                            }
                            '!' => {
                                if let Some('=') = buffer.peek() {
                                    buffer.next();
                                    output.push(Token::new(TokenType::Neq, Span::new(span_start, buffer.get_head())));
                                }
                                else {
                                    output.push(Token::new(TokenType::Not, Span::new(span_start, span_start)));
                                }
                            }
                            '>' => {
                                if let Some('=') = buffer.peek() {
                                    buffer.next();
                                    output.push(Token::new(TokenType::Gte, Span::new(span_start, buffer.get_head())));
                                }
                                else {
                                    output.push(Token::new(TokenType::Gt, Span::new(span_start, span_start)));
                                }
                            }
                            '<' => {
                                if let Some('=') = buffer.peek() {
                                    buffer.next();
                                    output.push(Token::new(TokenType::Lte, Span::new(span_start, buffer.get_head())));
                                }
                                else {
                                    output.push(Token::new(TokenType::Lt, Span::new(span_start, span_start)));
                                }
                            }
                            _ => { panic!("syntax error! unexpected character {} (todo: better error handling)", c); }
                        }
                    }
                    buffer.skip(char::is_whitespace);
                }
            }
            _ => {
                let mut buf = String::new();
                while let Some(c) = buffer.peek() {
                    if c == '\\' {
                        buffer.next();
                        if let Some('[') = buffer.peek() {
                            buffer.next();
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
                        buffer.next();
                    }
                }
                if buf.len() > 0 {
                    output.push(Token::new(TokenType::Literal(Literal::Text(buf)), Span::new(span_start, buffer.get_head())));
                }
            }
        }
    }
    output
}

