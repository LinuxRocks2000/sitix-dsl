mod lookahead;
use lookahead::*;
mod utility;
mod lexer;
use lexer::*;
mod ast;
mod parse;
mod inflate;
use inflate::*;
mod interpret;
use interpret::*;


// parsing works in three stages.
/*
   Stage 1: Lexer
    A knight travels through the land, accumulating much dust about him; it takes but an LL(1) interpreting step to scrub clean
    the shining armor within!

    Converts characters into Tokens, which are a thin wrapper over enum TokenType and an attached span (for debugging).
   Stage 2: Inflator
    Wherein embarks our hero (the language :P) to contrive distinction across the vast gulfs of tokens; producing meaning from chaos,
    storing Vec<Token>s in abstract structures to represent the shape of an actual sitix expression (recursively, of course!)

    sitix is a two-level language; you can think of it as essentially the inverse of the relationship between c and c preprocessor.
    this step grabs the preprocessing blocks out of a token stream and munges them together, storing the unparsed syntax (the actual
    DSL) inside the blocks. this is what allows for magic like [else] and enclosed-expressions.
   Stage 3: Parser
    A novice sees but rubble on uneven ground; a master sees a recursive grammar assembling trees to challenge any tower!

    This recursively transforms the inflated layer into a proper syntax tree. At this point all Tokens have been consumed and
    replaced with Statements, Expressions, and other such nastiness; we can convert to bytecode now, or just tree-walk interpret. Whew!
*/

fn parse_file(fname : impl AsRef<str>) {
    let contents = std::fs::read_to_string(fname.as_ref()).unwrap();
    let iter = contents.chars();
    let buffer = SimpleLLBuffer::new(iter);
    let tokens = lexer(buffer);
    let mut token_buffer = SimpleLLBuffer::new(tokens.into_iter());

    let mut inflated = SitixTree::root(&mut token_buffer).unwrap(); // the data structure here is significantly more useful to the final stage than a raw token stream

    let ast = inflated.parse().unwrap();

    println!("ast: {:#?}", ast);

    let mut interpreter = InterpreterState::new();
    println!("interpreter result: {}", ast.interpret(&mut interpreter).unwrap().to_string());
    println!("interpreter state: {:?}", interpreter);
}


fn main() {
    parse_file("test.stx");
}

// 1061, 1.5, 762
