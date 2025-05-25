// the visitor pattern is inexcusably bad, and I decline to use it. ever. for any reason. FUCK visitors.
// herein is a trait Parse and implementations over the entire syntax tree.
use crate::utility::*;
use crate::ast::*;
use crate::inflate::TreeChild;
use crate::inflate::BlockMode;
use crate::error::{ Error, SitixResult };


#[derive(Debug, Clone)]
pub struct TokenReader {
    tokens : Vec<Token>,
    last_span : Span,
    span_track_start : Option<usize>,
    span_head_current : usize,
    head : usize
}


impl TokenReader {
    pub fn new(tokens : Vec<Token>) -> Self {
        TokenReader {
            tokens,
            last_span : Span::new(0, 0, "unknown_file".to_string()),
            span_track_start : None,
            span_head_current : 0,
            head : 0
        }
    }

    pub fn next(&mut self) -> SitixResult<Token> {
        self.last_span = self.peek()?.span;
        self.span_head_current = self.last_span.end_char;
        if let None = self.span_track_start {
            self.span_track_start = Some(self.last_span.start_char);
        }
        self.head += 1;
        self.tokens.get(self.head - 1).ok_or(Error::unexpected_eof(self.last_span.clone())).cloned()
    }

    pub fn peek(&self) -> SitixResult<Token> {
        self.tokens.get(self.head).ok_or(Error::unexpected_eof(self.last_span.clone())).cloned()
    }

    pub fn get_last_span(&self) -> Span {
        self.last_span.clone()
    }

    fn pcheck(&mut self, thing : TokenType) -> SitixResult<()> {
        let tok = self.next()?;
        if tok.tp == thing {
            Ok(())
        }
        else {
            Err(Error::expected(&[thing], tok))
        }
    }

    fn unexpected_eof(&self) -> Error {
        Error::unexpected_eof(self.last_span.clone())
    }

    fn start_span_tracker(&mut self) {
        self.span_track_start = None;
    }

    fn get_tracked_span(&self) -> Option<Span> {
        Some(Span::new(self.span_track_start?, self.span_head_current, self.last_span.filename.clone()))
    }
}


impl crate::inflate::SitixTree {
    fn parse_table_entry(&mut self) -> SitixResult<TableEntry> {
        let labelish = self.parse_expression()?;
        if let Ok(tok) = self.content.peek() {
            if let TokenType::Colon = tok.tp {
                self.content.next()?;
                let expr = self.parse_expression()?;
                return Ok(TableEntry {
                    content : Box::new(expr),
                    label : Some(Box::new(labelish))
                });
            }
        }
        Ok(TableEntry {
            content : Box::new(labelish),
            label : None
        })
    }

    fn parse_primary(&mut self) -> SitixResult<Expression> {
        if let Ok(tok) = self.content.next() {
            Ok(match tok.tp {
                TokenType::Literal(Literal::Ident(ident)) => Expression::UnboundVariableAccess(tok.span, ident),
                TokenType::Literal(lit) => Expression::Literal(tok.span, lit),
                TokenType::True => Expression::True(tok.span),
                TokenType::False => Expression::False(tok.span),
                TokenType::Nil => Expression::Nil(tok.span),
                TokenType::LeftParen => {
                    let expr = self.parse_expression()?;
                    self.content.pcheck(TokenType::RightParen)?;
                    Expression::Grouping(Box::new(expr))
                },
                TokenType::LeftBrace => {
                    let block = self.parse_block()?;
                    self.content.pcheck(TokenType::RightBrace)?;
                    Expression::Braced(Box::new(block))
                },
                TokenType::LeftBracket => {
                    let mut table = vec![];
                    let first = self.content.peek()?.span;
                    loop {
                        if let TokenType::RightBracket = self.content.peek()?.tp {
                            break;
                        }
                        table.push(self.parse_table_entry()?);
                        let tok = self.content.next()?;
                        match tok.tp { // TODO: figure out a more ergonomic way to write this (maybe a .check() function?)
                            TokenType::RightBracket => {
                                break;
                            },
                            TokenType::Comma => {},
                            _ => {
                                return Err(Error::expected(&[TokenType::RightBracket, TokenType::Comma], tok));
                            }
                        }
                    }
                    Expression::Table(first.merge(self.content.get_last_span()), table)
                },
                TokenType::If => {
                    let if_expr = self.parse_expression()?;
                    let main_body = self.parse_expression()?;
                    let else_body = if let Ok(tok) = self.content.peek() {
                        if let TokenType::Else = tok.tp {
                            self.content.next()?;
                            Some(Box::new(self.parse_expression()?))
                        }
                        else {
                            None
                        }
                    } else {
                        if let Some((BlockMode::Else, children)) = self.children.get_mut(1) {
                            let ret : SitixResult<Vec<SitixExpression>> = children.iter_mut().map(|thing| {
                                match thing {
                                    TreeChild::Text(text, span) => Ok(SitixExpression::Text(text.clone(), span.clone())),
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
                    Expression::IfBranch(tok.span, Box::new(if_expr), Box::new(main_body), else_body)
                },
                TokenType::While => {
                    let conditional_expression = self.parse_expression()?;
                    let body_expression = self.parse_expression()?;
                    return Ok(Expression::While(tok.span, Box::new(conditional_expression), Box::new(body_expression)));
                },
                TokenType::Each => {
                    let array_expression = self.parse_expression()?;
                    self.content.pcheck(TokenType::DashTo)?;
                    let ident = self.content.next()?;
                    if let TokenType::Literal(Literal::Ident(ident)) = ident.tp {
                        let secondary_ident = if let Ok(tok) = self.content.peek() {
                            if let TokenType::Comma = tok.tp {
                                self.content.next()?;
                                let i = self.content.next()?;
                                if let TokenType::Literal(Literal::Ident(i)) = i.tp {
                                    Some(i)
                                }
                                else {
                                    return Err(Error::expected_abstract("identifier", i.span));
                                }
                            }
                            else { None }
                        } else { None };
                        let eval_expression = self.parse_expression()?;
                        Expression::UnboundEach(tok.span, Box::new(array_expression), ident, secondary_ident, Box::new(eval_expression))
                    }
                    else {
                        return Err(Error::expected_abstract("identifier", ident.span));
                    }
                }
                TokenType::Fun => {
                    self.content.pcheck(TokenType::LeftParen)?;
                    let args = self.parse_csl(TokenType::RightParen)?;
                    let mut args_to = vec![];
                    for arg in args {
                        if let Expression::UnboundVariableAccess(span, var) = arg {
                            args_to.push((var, span));
                        }
                        else {
                            return Err(Error::bad_argument(tok));
                        }
                    }
                    return Ok(Expression::UnboundFunction(tok.span, args_to, Box::new(self.parse_expression()?)));
                }
                _ => { return Err(Error::expected_abstract("literal, boolean, parenthesized expression, or nil", tok.span)); }
            })
        }
        else { // this *would* be an eof, but there's a chance for recovery!
            if let Some((BlockMode::Main, children)) = self.children.get_mut(0) {
                let ret : SitixResult<Vec<SitixExpression>> = children.iter_mut().map(|thing| {
                    match thing {
                        TreeChild::Text(text, span) => Ok(SitixExpression::Text(text.clone(), span.clone())),
                        TreeChild::Tree(tree) => tree.parse() // AHAHAHAA RECURSION HAHAAHA BWAHAALKHASDLFHASDLFH
                    }
                }).collect();
                Ok(Expression::SitixExpression(ret?))
            }
            else {
                Err(self.content.unexpected_eof())
            }
        }
    }

    fn parse_csl(&mut self, end : TokenType) -> SitixResult<Vec<Expression>> { // Comma Separated List
        let mut out = vec![];
        loop {
            if let Ok(tok) = self.content.peek() {
                if tok.tp == end {
                    self.content.next()?;
                    break;
                }
            }
            out.push(self.parse_expression()?);
            let tok = self.content.next()?;
            match tok.tp {
                TokenType::Comma => {}
                ref otherwise => {
                    if *otherwise == end {
                        break;
                    }
                    return Err(Error::expected(&[TokenType::Comma, end], tok));
                }
            }
        }
        Ok(out)
    }

    fn parse_dotaccess(&mut self) -> SitixResult<Expression> {
        let mut out = self.parse_call()?;
        while let Ok(tok) = self.content.peek() {
            if let TokenType::Dot = tok.tp {
                self.content.next()?;
                let id = self.content.next()?;
                if let TokenType::Literal(Literal::Ident(ident)) = id.tp {
                    out = Expression::DotAccess(Box::new(out), ident);
                }
                else {
                    return Err(Error::expected_abstract("literal", id.span));
                }
            }
            else if let TokenType::LeftParen = tok.tp {
                self.content.next()?;
                let mut args = self.parse_csl(TokenType::RightParen)?;
                if let Err(_) = self.content.peek() {
                    if self.children.len() > 0 {
                        args.push(self.parse_expression()?);
                    }
                }
                out = Expression::Call(Box::new(out), args);
            }
            else {
                break;
            }
        }
        Ok(out)
    }

    fn parse_call(&mut self) -> SitixResult<Expression> {
        let lhs = self.parse_primary()?;
        if let Ok(tok) = self.content.peek() { // TODO: come up with a more ergonomic way to write this very common pattern
            if let TokenType::LeftParen = tok.tp {
                self.content.next()?;
                let mut args = self.parse_csl(TokenType::RightParen)?;
                if let Err(_) = self.content.peek() {
                    if self.children.len() > 0 {
                        args.push(self.parse_expression()?);
                    }
                }
                return Ok(Expression::Call(Box::new(lhs), args));
            }
        }
        Ok(lhs)
    }

    fn parse_unary(&mut self) -> SitixResult<Expression> {
        if let Ok(tok) = self.content.peek() {
            Ok(match tok.tp {
                TokenType::Not => {
                    self.content.next()?;
                    Expression::Unary(Unary::Not(tok.span, Box::new(self.parse_unary()?)))
                },
                TokenType::Minus => {
                    self.content.next()?;
                    Expression::Unary(Unary::Negative(tok.span, Box::new(self.parse_unary()?)))
                },
                _ => {
                    self.parse_dotaccess()?
                }
            })
        }
        else {
            self.parse_dotaccess()
        }
    }

    fn parse_factor(&mut self) -> SitixResult<Expression> {
        let lhs = self.parse_unary()?;
        if let Ok(tok) = self.content.peek() {
            Ok(match tok.tp {
                TokenType::Star => {
                    self.content.next()?;
                    Expression::Binary(Binary::Mul(Box::new(lhs), Box::new(self.parse_factor()?)))
                },
                TokenType::Slash => {
                    self.content.next()?;
                    Expression::Binary(Binary::Div(Box::new(lhs), Box::new(self.parse_factor()?)))
                },
                TokenType::Modulo => {
                    self.content.next()?;
                    Expression::Binary(Binary::Mod(Box::new(lhs), Box::new(self.parse_factor()?)))
                }
                _ => lhs
            })
        }
        else {
            Ok(lhs)
        }
    }

    fn parse_term(&mut self) -> SitixResult<Expression> {
        let lhs = self.parse_factor()?;
        if let Ok(tok) = self.content.peek() {
            Ok(match tok.tp {
                TokenType::Plus => {
                    self.content.next()?;
                    Expression::Binary(Binary::Add(Box::new(lhs), Box::new(self.parse_term()?)))
                },
                TokenType::Minus => {
                    self.content.next()?;
                    Expression::Binary(Binary::Sub(Box::new(lhs), Box::new(self.parse_term()?)))
                },
                _ => lhs
            })
        }
        else {
            Ok(lhs)
        }
    }

    fn parse_comparison(&mut self) -> SitixResult<Expression> {
        let lhs = self.parse_term()?;
        if let Ok(tok) = self.content.peek() {
            Ok(match tok.tp {
                TokenType::Gt => {
                    self.content.next()?;
                    Expression::Binary(Binary::Gt(Box::new(lhs), Box::new(self.parse_term()?)))
                },
                TokenType::Gte => {
                    self.content.next()?;
                    Expression::Binary(Binary::Gte(Box::new(lhs), Box::new(self.parse_term()?)))
                },
                TokenType::Lt => {
                    self.content.next()?;
                    Expression::Binary(Binary::Lt(Box::new(lhs), Box::new(self.parse_term()?)))
                },
                TokenType::Lte => {
                    self.content.next()?;
                    Expression::Binary(Binary::Lte(Box::new(lhs), Box::new(self.parse_term()?)))
                },
                _ => lhs
            })
        }
        else {
            Ok(lhs)
        }
    }

    fn parse_logic(&mut self) -> SitixResult<Expression> {
        let lhs = self.parse_comparison()?;
        if let Ok(tok) = self.content.peek() {
            Ok(match tok.tp {
                TokenType::And => {
                    self.content.next()?;
                    Expression::Binary(Binary::And(Box::new(lhs), Box::new(self.parse_logic()?)))
                },
                TokenType::Or => {
                    self.content.next()?;
                    Expression::Binary(Binary::Or(Box::new(lhs), Box::new(self.parse_logic()?)))
                }
                _ => lhs
            })
        }
        else {
            Ok(lhs)
        }
    }

    fn parse_equality(&mut self) -> SitixResult<Expression> {
        let lhs = self.parse_logic()?;
        if let Ok(tok) = self.content.peek() {
            Ok(match tok.tp {
                TokenType::Neq => {
                    self.content.next()?;
                    Expression::Binary(Binary::Nequals(Box::new(lhs), Box::new(self.parse_logic()?)))
                },
                TokenType::EqEq => {
                    self.content.next()?;
                    Expression::Binary(Binary::Equals(Box::new(lhs), Box::new(self.parse_logic()?)))
                },
                _ => lhs
            })
        }
        else {
            Ok(lhs)
        }
    }

    fn parse_assignment(&mut self) -> SitixResult<Expression> {
        let expr = self.parse_equality()?;
        if let Ok(tok) = self.content.peek() {
            if let TokenType::Eq = tok.tp {
                self.content.next()?;
                let value = self.parse_assignment()?;
                return Ok(Expression::Assignment(Box::new(expr), Box::new(value)));
            }
        }
        Ok(expr)
    }

    fn parse_expression(&mut self) -> SitixResult<Expression> {
        self.parse_assignment()
    }

    fn parse_statement(&mut self) -> SitixResult<Statement> {
        if let Ok(outer_tok) = self.content.peek() {
            match &outer_tok.tp {
                TokenType::Debugger => {
                    self.content.next()?;
                    return Ok(Statement::Debugger(outer_tok.span));
                },
                TokenType::Let | TokenType::Global => {
                    self.content.next()?;
                    let pattern = match outer_tok.tp {
                        TokenType::Let => Statement::UnboundLetAssign,
                        TokenType::Global => Statement::UnboundGlobalAssign,
                        _ => panic!("unreachable")
                    };
                    let tok = self.content.next()?;
                    if let TokenType::Literal(Literal::Ident(ident)) = tok.tp {
                        self.content.pcheck(TokenType::Eq)?;
                        return Ok(
                            pattern(tok.span, ident, Box::new(self.parse_expression()?))
                        );
                    }
                    else {
                        return Err(Error::expected_abstract("identifier", tok.span));
                    }
                }
                _ => {}
            }
        }
        Ok(Statement::Expression(Box::new(self.parse_expression()?)))
    }

    fn parse_block(&mut self) -> SitixResult<Block> {
        // a block is a semicolon-separated list of statements,
        // with an optional tail
        let mut inner = Vec::new();
        let mut tail;
        self.content.start_span_tracker();
        loop {
            tail = Some(self.parse_statement()?);
            match self.content.peek() { // check this before doing semicolon checks; if the output is ended without
                                        // a semicolon, the preceding statement is a tail
                Err(_) => {
                    break;
                },
                Ok(tok) => {
                    if let TokenType::RightBrace = tok.tp {
                        break;
                    }
                }
            }
            self.content.pcheck(TokenType::Semicolon)?; // if we *didn't* find an eob, the next token *must* be a semicolon!
            inner.push(tail.unwrap());
            tail = None;
            match self.content.peek() {
                Err(_) => {
                    break;
                },
                Ok(tok) => {
                    if let TokenType::RightBrace = tok.tp {
                        break;
                    }
                }
            }
        }
        Ok(Block {
            inner,
            tail,
            span : if let Some(span) = self.content.get_tracked_span() { span } else { Span::identity() }
        })
    }

    pub fn parse(&mut self) -> SitixResult<SitixExpression> { // why use SitixExpression here?
        // I flipflopped on this a bit, but in the
        // end it's simpler to get a sitix expression from this function
        // than transform a Vec<Statement> into a SitixExpression. It is
        // guaranteed to *always* return SitixExpression::Block(_).
        Ok(SitixExpression::Block(self.parse_block()?))
    }
}

