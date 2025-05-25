mod utility;
mod lexer;
mod ast;
mod parse;
mod inflate;
use inflate::*;
mod interpret;
use interpret::*;
mod resolve;
use resolve::*;
mod ffi;
use ffi::*;
mod error;
use error::SitixResult;


// parsing works in three stages.
/*
   Stage 1: Lexer
    A knight travels through the land, accumulating much dust about him; it takes but an LL(1) interpreting step to scrub clean
    the shining armor within!

    Code is dirty; the lexer cleans it up into a nice and simple Token-ized representation.
   Stage 2: Inflator
    Wherein embarks our hero (the language :P) to contrive distinction across the vast gulfs of tokens; producing meaning from chaos,
    storing Vec<Token>s in abstract structures to represent the shape of an actual sitix expression (recursively, of course!)

    sitix is a two-level language; you can think of it as essentially the inverse of the relationship between c and c preprocessor.
    this step grabs the preprocessing blocks out of a token stream and munges them together, storing the unparsed syntax (the actual
    DSL) inside the blocks. this is what allows for magic like [else] and enclosed-expressions.
   Stage 3: Parser
    A novice sees but rubble on uneven ground; a master sees a recursive grammar assembling trees to challenge any tower!

    This recursively transforms the inflated layer into a proper syntax tree. At this point all Tokens have been consumed and
    replaced with Statements, Expressions, and other such nastiness.
   Stage 4: Lexical Binding
    A novice, on a hike through the forest, passed through a clearing. Several minutes later, after much twisting and turning, he was
    surprised to walk through a clearing exactly like it. He brought this news to a master, thinking himself the discoverer of some
    great mystery- "who is to say, Wise Master, whether I walked through the same clearing, or a different one? Is it not silly
    to presume that the distinction is meaningful?"
    The master did not reply, but conjured a great wind, which brought many sheets of numbered paper to fly through the forest and alight
    in the enigmatic clearing. With this, the novice was enlightened.

    There are a bunch of edge cases where the obvious lexical meaning of a variable is quite different from what the
    interpreter actually sees. The binding step discards identifiers and ensures that every variable creation and
    access are uniquely indexable by the interpreter - simply by assigning each variable a unique number.
*/

use std::sync::Arc;


pub fn parse_file(fname : impl ToString) -> SitixResult<Data> {
    let file = lexer::FileReader::open(fname);
    let tokens = lexer::lexer(file)?;

    println!("tokens: {:?}", tokens);

    let mut token_buffer = parse::TokenReader::new(tokens);
    let mut inflated = SitixTree::root(&mut token_buffer)?; // the data structure here is significantly more useful to the final stage than a raw token stream

    println!("inflate: {:#?}", inflated);

    let ast = inflated.parse()?;

    println!("ast: {:#?}", ast);

    let mut ffi = ForeignFunctionInterface::new();
    ffi.add_standard_api();
    let ffi = Arc::new(ffi);

    let mut resolver = ResolverState::new(ffi.clone());
    let ast = ast.resolve(&mut resolver);

    let mut interpreter = InterpreterState::new(resolver, ffi.clone());
    Ok(ast.interpret(&mut interpreter).unwrap())
}


fn main() {
    println!("{}", match parse_file("test.stx") {
        Err(e) => format!("{:?}", e),
        Ok(s) => s.to_string()
    });
}

// 1061, 1.5, 762
