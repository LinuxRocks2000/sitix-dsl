// turn a token stream into a deflattened sitix tree

use crate::utility::*;
use crate::lookahead::*;
use crate::parse::{ ParseResult, ParseError };
use std::vec::IntoIter;


#[derive(Debug)]
pub enum TreeChild {
    Text(String),
    Tree(SitixTree)
}


#[derive(Debug)]
pub struct SitixTree {
    pub content : SimpleLLBuffer<Token, IntoIter<Token>>, // main body
    pub children : Vec<TreeChild> // this will be parsed as an expression if necessary
}


impl SitixTree {
    pub fn root(tokens : &mut impl LookaheadBuffer<Token>) -> ParseResult<SitixTree> {
        Ok(SitixTree {
            content : SimpleLLBuffer::new(Vec::new().into_iter()),
            children : Self::parse_contained(tokens)?
        })
    }

    fn parse_contained(tokens : &mut impl LookaheadBuffer<Token>) -> ParseResult<Vec<TreeChild>> {
        let mut ret = vec![];
        'mainloop: while let Some(token) = tokens.next() {
            if let TokenType::Literal(Literal::Text(text)) = token.tp {
                ret.push(TreeChild::Text(text));
            }
            else if let TokenType::BlockOpen = token.tp {
                let mut block_contents = vec![];
                let mut block_children = vec![];
                'parse_block: loop {
                    let inner_token = tokens.next().ok_or(ParseError::UnexpectedEof)?;
                    if let TokenType::BlockClose(extended) = inner_token.tp {
                        if extended {
                            block_children = Self::parse_contained(tokens)?;
                        }
                        break 'parse_block;
                    }
                    else {
                        block_contents.push(inner_token);
                    }
                }
                if block_contents.len() == 1 {
                    if let TokenType::Slash = block_contents[0].tp {
                        break 'mainloop;
                    }
                }
                ret.push(TreeChild::Tree(SitixTree {
                    content : SimpleLLBuffer::new(block_contents.into_iter()),
                    children : block_children
                }));
            }
        }
        Ok(ret)
    }
}
