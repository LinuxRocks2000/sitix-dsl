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
pub enum BlockMode {
    Main,
    Else
}

#[derive(Debug)]
enum BlockTermMode { // internal data structure describing how a block was terminated.
    Closing, // a normal [/] or EOF, this block is fully done
    Continue(BlockMode) // [else] and co
}


#[derive(Debug)]
pub struct SitixTree { // contains a program and one or more bodies which are interpreted in various ways by the parser.
    // for instance, the first body (which always matches BlockMode::Main) might be interpreted directly as an Expression if one is needed
    pub content : SimpleLLBuffer<Token, IntoIter<Token>>, // main body
    pub children : Vec<(BlockMode, Vec<TreeChild>)> // this will be parsed as an expression if necessary
}


impl SitixTree {
    pub fn root(tokens : &mut impl LookaheadBuffer<Token>) -> ParseResult<SitixTree> {
        Ok(SitixTree {
            content : SimpleLLBuffer::new(Vec::new().into_iter()),
            children : vec![(BlockMode::Main, Self::parse_contained(tokens)?.1)]
        })
    }

    fn parse_contained(tokens : &mut impl LookaheadBuffer<Token>) -> ParseResult<(BlockTermMode, Vec<TreeChild>)> {
        let mut ret = vec![];
        let mut term_mode = BlockTermMode::Closing;
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
                            let (ext_mode, ext_data) = Self::parse_contained(tokens)?;
                            block_children.push((BlockMode::Main, ext_data));
                            if let BlockTermMode::Continue(BlockMode::Else) = ext_mode {
                                if let (BlockTermMode::Closing, block) = Self::parse_contained(tokens)? {
                                    block_children.push((BlockMode::Else, block));
                                }
                                else {
                                    return Err(ParseError::ExpectedElseContents);
                                }
                            }
                        }
                        break 'parse_block;
                    }
                    else {
                        block_contents.push(inner_token);
                    }
                }
                if block_contents.len() == 1 {
                    match block_contents[0].tp {
                        TokenType::Slash => {
                            break 'mainloop;
                        },
                        TokenType::Else => {
                            term_mode = BlockTermMode::Continue(BlockMode::Else);
                            break 'mainloop;
                        },
                        _ => {}
                    }
                }
                ret.push(TreeChild::Tree(SitixTree {
                    content : SimpleLLBuffer::new(block_contents.into_iter()),
                    children : block_children
                }));
            }
        }
        Ok((term_mode, ret))
    }
}
