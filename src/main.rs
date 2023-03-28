//pub mod compiler;
//pub mod compiler;
pub mod core;
pub mod interpreter;

use std::fs::File;
use std::io::prelude::*;

fn read_file(filename: &str) -> Result<String, std::io::Error> {
    let mut file = File::open(filename)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}

fn main() {
    let contents = read_file("input.nk").unwrap();
    //let contents = "public fn main() -> void\n{\nlet:int a = 3;}";
    //println!("Input: {}", contents);

    let tokens = core::lexer::lexer(&contents);
    //println!("Tokens: {:?}", tokens);
    //let ast = core::parser_new::parse::Parser::new(tokens).parse();
    //println!("{:?}", ast);
    // Pass contents to the lexer here
    let ast = core::parser_new::parse::Parser::new(&tokens).parse();
    //println!("AST Tree: {:?}", ast.unwrap());

    //let compiled = compiler::compile::compile_and_run(ast.unwrap());
    let mut interpreter = interpreter::Interpreter::new();
    interpreter.run(ast.unwrap());
}
