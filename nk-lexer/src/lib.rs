mod errors;
mod lex;
pub mod lex_new;
pub mod lex_new_new;
mod tokens;
pub mod tokens_new;

pub use lex::lexer;
pub use tokens::*;

// benchmark between the two lexers
