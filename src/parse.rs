// the visitor pattern is inexcusably bad, and I decline to use it. ever. for any reason. FUCK visitors.
// herein is a trait Parse and implementations over the entire syntax tree.
use crate::lookahead::*;
use thiserror::Error;
use crate::utility::*;
use crate::ast::*;
use crate::inflate::TreeChild;
use crate::inflate::BlockMode;


#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Unexpected end-of-file reached during parsing!")]
    UnexpectedEof,
    #[error("Expected {0:?}, got {1:?}")]
    Expected (String, String),
    #[error("Expected else contents")]
    ExpectedElseContents,
    #[error("Bad Argument")]
    BadArgument
}

pub type ParseResult<T> = Result<T, ParseError>;



impl crate::inflate::SitixTree {
    fn pcheck(&mut self, thing : TokenType) -> ParseResult<()> {
        let val = self.content.next().ok_or(ParseError::UnexpectedEof)?.tp;
        if val == thing {
            Ok(())
        }
        else {
            Err(ParseError::Expected(thing.to_string(), val.to_string()))
        }
    }

    fn parse_primary(&mut self) -> ParseResult<Expression> {
        if let Some(tok) = self.content.next() {
            let tok = tok.tp;
            Ok(match tok {
                TokenType::Literal(Literal::Ident(ident)) => Expression::VariableAccess(ident),
                TokenType::Literal(lit) => Expression::Literal(lit),
                TokenType::True => Expression::True,
                TokenType::False => Expression::False,
                TokenType::Nil => Expression::Nil,
                TokenType::LeftParen => {
                    let expr = self.parse_expression()?;
                    self.pcheck(TokenType::RightParen)?;
                    Expression::Grouping(Box::new(expr))
                },
                TokenType::LeftBrace => {
                    let block = self.parse_block()?;
                    self.pcheck(TokenType::RightBrace)?;
                    Expression::Braced(Box::new(block))
                },
                TokenType::LeftBracket => {
                    let mut table = vec![];
                    loop {
                        if let TokenType::RightBracket = self.content.peek().ok_or(ParseError::UnexpectedEof)?.tp {
                            break;
                        }
                        table.push(self.parse_expression()?);
                        let tok = self.content.next().ok_or(ParseError::UnexpectedEof)?;
                        match tok.tp {
                            TokenType::RightBracket => {
                                break;
                            },
                            TokenType::Comma => {},
                            _ => {
                                return Err(ParseError::Expected("] or ,".to_string(), tok.tp.to_string()));
                            }
                        }
                    }
                    Expression::Table(table)
                },
                TokenType::If => {
                    let if_expr = self.parse_expression()?;
                    let main_body = self.parse_expression()?;
                    let else_body = if let Some(tok) = self.content.peek() {
                        if let TokenType::Else = tok.tp {
                            self.content.next();
                            Some(Box::new(self.parse_expression()?))
                        }
                        else {
                            None
                        }
                    } else {
                        if let Some((BlockMode::Else, children)) = self.children.get_mut(1) {
                            let ret : ParseResult<Vec<SitixExpression>> = children.iter_mut().map(|thing| {
                                match thing {
                                    TreeChild::Text(text) => Ok(SitixExpression::Text(text.clone())),
                                    TreeChild::Tree(tree) => tree.parse() // AHAHAHAA RECURSION HAHAAHA BWAHAALKHASDLFHASDLFH
                                                                          // [a bit later] sometimes I read comments I wrote and then I feel sad
                                }
                            }).collect();
                            Some(Box::new(Expression::SitixExpression(ret?)))
                        }
                        else {
                            None
                        }
                    };
                    Expression::IfBranch(Box::new(if_expr), Box::new(main_body), else_body)
                },
                TokenType::While => {
                    let conditional_expression = self.parse_expression()?;
                    let body_expression = self.parse_expression()?;
                    return Ok(Expression::While(Box::new(conditional_expression), Box::new(body_expression)));
                },
                TokenType::Fun => {
                    self.pcheck(TokenType::LeftParen)?;
                    let args = self.parse_csl(TokenType::RightParen)?;
                    let mut args_to = vec![];
                    for arg in args {
                        if let Expression::VariableAccess(var) = arg {
                            args_to.push(var);
                        }
                        else {
                            return Err(ParseError::BadArgument);
                        }
                    }
                    return Ok(Expression::Function(args_to, Box::new(self.parse_expression()?)));
                }
                _ => { return Err(ParseError::Expected("literal, boolean, parenthesized expression, or nil".to_string(), tok.to_string())); }
            })
        }
        else { // this *would* be an eof, but there's a chance for recovery!
            if let Some((BlockMode::Main, children)) = self.children.get_mut(0) {
                let ret : ParseResult<Vec<SitixExpression>> = children.iter_mut().map(|thing| {
                    match thing {
                        TreeChild::Text(text) => Ok(SitixExpression::Text(text.clone())),
                        TreeChild::Tree(tree) => tree.parse() // AHAHAHAA RECURSION HAHAAHA BWAHAALKHASDLFHASDLFH
                    }
                }).collect();
                Ok(Expression::SitixExpression(ret?))
            }
            else {
                Err(ParseError::UnexpectedEof)
            }
        }
    }

    fn parse_csl(&mut self, end : TokenType) -> ParseResult<Vec<Expression>> { // Comma Separated List
        let mut out = vec![];
        loop {
            if let Some(tok) = self.content.peek() {
                if tok.tp == end {
                    self.content.next();
                    break;
                }
            }
            out.push(self.parse_expression()?);
            let tok = self.content.next().ok_or(ParseError::UnexpectedEof)?;
            match tok.tp {
                TokenType::Comma => {}
                ref otherwise => {
                    if *otherwise == end {
                        break;
                    }
                    return Err(ParseError::Expected(format!("comma or {}", end.to_string()), tok.tp.to_string()));
                }
            }
        }
        Ok(out)
    }

    fn parse_call(&mut self) -> ParseResult<Expression> {
        let lhs = self.parse_primary()?;
        if let Some(tok) = self.content.peek() {
            if let TokenType::LeftParen = tok.tp {
                self.content.next();
                let args = self.parse_csl(TokenType::RightParen)?;
                return Ok(Expression::Call(Box::new(lhs), args));
            }
        }
        Ok(lhs)
    }

    fn parse_unary(&mut self) -> ParseResult<Expression> {
        if let Some(tok) = self.content.peek() {
            Ok(match tok.tp {
                TokenType::Not => {
                    self.content.next();
                    Expression::Unary(Unary::Not(Box::new(self.parse_unary()?)))
                },
                TokenType::Minus => {
                    self.content.next();
                    Expression::Unary(Unary::Negative(Box::new(self.parse_unary()?)))
                },
                _ => {
                    self.parse_call()?
                }
            })
        }
        else {
            self.parse_call()
        }
    }

    fn parse_factor(&mut self) -> ParseResult<Expression> {
        let lhs = self.parse_unary()?;
        if let Some(tok) = self.content.peek() {
            Ok(match tok.tp {
                TokenType::Star => {
                    self.content.next();
                    Expression::Binary(Binary::Mul(Box::new(lhs), Box::new(self.parse_factor()?)))
                },
                TokenType::Slash => {
                    self.content.next();
                    Expression::Binary(Binary::Div(Box::new(lhs), Box::new(self.parse_factor()?)))
                },
                TokenType::Modulo => {
                    self.content.next();
                    Expression::Binary(Binary::Mod(Box::new(lhs), Box::new(self.parse_factor()?)))
                }
                _ => lhs
            })
        }
        else {
            Ok(lhs)
        }
    }

    fn parse_term(&mut self) -> ParseResult<Expression> {
        let lhs = self.parse_factor()?;
        if let Some(tok) = self.content.peek() {
            Ok(match tok.tp {
                TokenType::Plus => {
                    self.content.next();
                    Expression::Binary(Binary::Add(Box::new(lhs), Box::new(self.parse_term()?)))
                },
                TokenType::Minus => {
                    self.content.next();
                    Expression::Binary(Binary::Sub(Box::new(lhs), Box::new(self.parse_term()?)))
                },
                _ => lhs
            })
        }
        else {
            Ok(lhs)
        }
    }

    fn parse_comparison(&mut self) -> ParseResult<Expression> {
        let lhs = self.parse_term()?;
        if let Some(tok) = self.content.peek() {
            Ok(match tok.tp {
                TokenType::Gt => {
                    self.content.next();
                    Expression::Binary(Binary::Gt(Box::new(lhs), Box::new(self.parse_term()?)))
                },
                TokenType::Gte => {
                    self.content.next();
                    Expression::Binary(Binary::Gte(Box::new(lhs), Box::new(self.parse_term()?)))
                },
                TokenType::Lt => {
                    self.content.next();
                    Expression::Binary(Binary::Lt(Box::new(lhs), Box::new(self.parse_term()?)))
                },
                TokenType::Lte => {
                    self.content.next();
                    Expression::Binary(Binary::Lte(Box::new(lhs), Box::new(self.parse_term()?)))
                },
                _ => lhs
            })
        }
        else {
            Ok(lhs)
        }
    }

    fn parse_logic(&mut self) -> ParseResult<Expression> {
        let lhs = self.parse_comparison()?;
        if let Some(tok) = self.content.peek() {
            Ok(match tok.tp {
                TokenType::And => {
                    self.content.next();
                    Expression::Binary(Binary::And(Box::new(lhs), Box::new(self.parse_logic()?)))
                },
                TokenType::Or => {
                    self.content.next();
                    Expression::Binary(Binary::Or(Box::new(lhs), Box::new(self.parse_logic()?)))
                }
                _ => lhs
            })
        }
        else {
            Ok(lhs)
        }
    }

    fn parse_equality(&mut self) -> ParseResult<Expression> {
        let lhs = self.parse_logic()?;
        if let Some(tok) = self.content.peek() {
            Ok(match tok.tp {
                TokenType::Neq => {
                    self.content.next();
                    Expression::Binary(Binary::Nequals(Box::new(lhs), Box::new(self.parse_logic()?)))
                },
                TokenType::EqEq => {
                    self.content.next();
                    Expression::Binary(Binary::Equals(Box::new(lhs), Box::new(self.parse_logic()?)))
                },
                _ => lhs
            })
        }
        else {
            Ok(lhs)
        }
    }

    fn parse_assignment(&mut self) -> ParseResult<Expression> {
        let expr = self.parse_equality()?;
        if let Some(tok) = self.content.peek() {
            if let TokenType::Eq = tok.tp {
                self.content.next();
                let value = self.parse_assignment()?;
                return Ok(Expression::Assignment(Box::new(expr), Box::new(value)));
            }
        }
        Ok(expr)
    }

    fn parse_expression(&mut self) -> ParseResult<Expression> {
        self.parse_assignment()
    }

    fn parse_statement(&mut self) -> ParseResult<Statement> {
        if let Some(tok) = self.content.peek() {
            match &tok.tp {
                TokenType::Debugger => {
                    self.content.next();
                    return Ok(Statement::Debugger);
                },
                TokenType::Let | TokenType::Global => {
                    self.content.next();
                    let pattern = match tok.tp {
                        TokenType::Let => Statement::LetAssign,
                        TokenType::Global => Statement::GlobalAssign,
                        _ => panic!("unreachable {:?}", tok.tp)
                    };
                    let tok = self.content.next().ok_or(ParseError::UnexpectedEof)?;
                    if let TokenType::Literal(Literal::Ident(ident)) = tok.tp {
                        self.pcheck(TokenType::Eq)?;
                        return Ok(
                            pattern(ident, Box::new(self.parse_expression()?))
                        );
                    }
                    else {
                        return Err(ParseError::Expected("identifier".to_string(), tok.tp.to_string()));
                    }
                }
                _ => {}
            }
        }
        Ok(Statement::Expression(Box::new(self.parse_expression()?)))
    }

    fn parse_block(&mut self) -> ParseResult<Block> {
        // a block is a semicolon-separated list of statements,
        // with an optional tail
        let mut inner = Vec::new();
        let mut tail;
        loop {
            tail = Some(self.parse_statement()?);
            match self.content.peek() { // check this before doing semicolon checks; if the output is ended without
                                        // a semicolon, the preceding statement is a tail
                None => {
                    break;
                },
                Some(tok) => {
                    if let TokenType::RightBrace = tok.tp {
                        break;
                    }
                }
            }
            self.pcheck(TokenType::Semicolon)?; // if we *didn't* find an eob, the next token *must* be a semicolon!
            inner.push(tail.unwrap());
            tail = None;
            match self.content.peek() {
                None => {
                    break;
                },
                Some(tok) => {
                    if let TokenType::RightBrace = tok.tp {
                        break;
                    }
                }
            }
        }
        Ok(Block {
            inner,
            tail
        })
    }

    pub fn parse(&mut self) -> ParseResult<SitixExpression> { // why use SitixExpression here?
        // I flipflopped on this a bit, but in the
        // end it's simpler to get a sitix expression from this function
        // than transform a Vec<Statement> into a SitixExpression. It is essentially
        // guaranteed to *always* return SitixExpression::Block(_).
        Ok(SitixExpression::Block(self.parse_block()?))
    }
}

