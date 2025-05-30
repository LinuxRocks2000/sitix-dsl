// turn a token stream into a deflattened sitix tree

use crate::utility::*;
use crate::parse::{ TokenReader };
use crate::error::{ SitixResult, Error };
use crate::utility::Span;


#[derive(Debug)]
pub enum TreeChild {
    Text(String, Span),
    Tree(SitixTree)
}


#[derive(Debug)]
pub enum BlockMode {
    Main,
    Else,
    List // the [,] block
}

#[derive(Debug)]
enum BlockTermMode { // internal data structure describing how a block was terminated.
    Closing, // a normal [/] or EOF, this block is fully done
    Continue(BlockMode) // [else] and co
}


#[derive(Debug)]
pub struct SitixTree { // contains a program and one or more bodies which are interpreted in various ways by the parser.
    // for instance, the first body (which always matches BlockMode::Main) might be interpreted directly as an Expression if one is needed
    pub content : TokenReader, // main body
    pub children : Vec<(BlockMode, Vec<TreeChild>)> // this will be parsed as an expression if necessary
}


impl SitixTree {
    pub fn root(tokens : &mut TokenReader) -> SitixResult<SitixTree> {
        Ok(SitixTree {
            content : TokenReader::new(Vec::new()),
            children : vec![(BlockMode::Main, Self::parse_contained(tokens)?.1)]
        })
    }

    fn parse_contained(tokens : &mut TokenReader) -> SitixResult<(BlockTermMode, Vec<TreeChild>)> {
        let mut ret = vec![];
        let mut term_mode = BlockTermMode::Closing;
        'mainloop: while let Ok(token) = tokens.next() {
            if let TokenType::Literal(Literal::Text(text)) = token.tp {
                ret.push(TreeChild::Text(text, token.span));
            }
            else if let TokenType::BlockOpen = token.tp {
                let mut block_contents = vec![];
                let mut block_children = vec![];
                'parse_block: loop {
                    let inner_token = tokens.next()?;
                    if let TokenType::BlockClose(extended) = inner_token.tp {
                        if extended {
                            let (ext_mode, ext_data) = Self::parse_contained(tokens)?;
                            block_children.push((BlockMode::Main, ext_data));
                            if let BlockTermMode::Continue(BlockMode::Else) = ext_mode {
                                if let (BlockTermMode::Closing, block) = Self::parse_contained(tokens)? {
                                    block_children.push((BlockMode::Else, block));
                                }
                                else {
                                    return Err(Error::expected_abstract("else contents", tokens.get_last_span()));
                                }
                            }
                            else if let BlockTermMode::Continue(BlockMode::List) = ext_mode {
                                loop {
                                    let (mode, data) = Self::parse_contained(tokens)?;
                                    block_children.push((BlockMode::List, data));
                                    if let BlockTermMode::Closing = mode {
                                        break;
                                    }
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
                        TokenType::Comma => {
                            term_mode = BlockTermMode::Continue(BlockMode::List);
                            break 'mainloop;
                        }
                        _ => {}
                    }
                }
                ret.push(TreeChild::Tree(SitixTree {
                    content : TokenReader::new(block_contents),
                    children : block_children
                }));
            }
        }
        Ok((term_mode, ret))
    }
}
