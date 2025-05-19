// the visitor pattern is inexcusably bad, and I decline to use it. ever. for any reason. FUCK visitors.
// herein is a trait Parse and implementations over the entire syntax tree.
use crate::lookahead::*;
use thiserror::Error;
use crate::utility::*;
use crate::ast::*;
use crate::inflate::TreeChild;


#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Unexpected end-of-file reached during parsing!")]
    UnexpectedEof,
    #[error("Expected {0:?}, got {1:?}")]
    Expected (String, String),

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
                TokenType::Literal(lit) => Expression::Literal(lit),
                TokenType::True => Expression::True,
                TokenType::False => Expression::False,
                TokenType::Nil => Expression::Nil,
                TokenType::LeftParen => {
                    let expr = self.parse_expression()?;
                    self.pcheck(TokenType::RightParen)?;
                    Expression::Grouping(Box::new(expr))
                },
                _ => { return Err(ParseError::Expected("literal, boolean, parenthesized expression, or nil".to_string(), tok.to_string())); }
            })
        }
        else { // this *would* be an eof, but there's a chance for recovery!
            if self.children.len() > 0 {
                let ret : ParseResult<Vec<SitixExpression>> = self.children.iter_mut().map(|thing| {
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
                    self.parse_primary()?
                }
            })
        }
        else {
            self.parse_primary()
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
                }
                _ => lhs
            })
        }
        else {
            Ok(lhs)
        }
    }

    fn parse_equality(&mut self) -> ParseResult<Expression> {
        let lhs = self.parse_comparison()?;
        if let Some(tok) = self.content.peek() {
            Ok(match tok.tp {
                TokenType::Neq => {
                    self.content.next();
                    Expression::Binary(Binary::Nequals(Box::new(lhs), Box::new(self.parse_comparison()?)))
                },
                TokenType::EqEq => {
                    self.content.next();
                    Expression::Binary(Binary::Equals(Box::new(lhs), Box::new(self.parse_comparison()?)))
                },
                _ => lhs
            })
        }
        else {
            Ok(lhs)
        }
    }

    fn parse_expression(&mut self) -> ParseResult<Expression> {
        self.parse_equality()
    }

    fn parse_statement(&mut self) -> ParseResult<Statement> {
        // TODO: implement statements that aren't just raw expressions
        Ok(Statement::Expression(Box::new(self.parse_expression()?)))
    }

    fn parse_block(&mut self) -> ParseResult<Block> {
        // a block is a semicolon-separated list of statements,
        // with an optional tail
        // TODO: implement semicolon-sep (right now it just defaults to tail)
        Ok(Block {
            inner : vec![], // this needs to be an actual vec of statements
            tail : Some(self.parse_statement()?)
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

