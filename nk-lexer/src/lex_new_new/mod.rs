mod errors;

mod identifier;
mod symbol;
mod value;

use errors::{LexError, LexcialError};

use std::iter::Peekable;
use std::str::Chars;

use inksac::{Color, Style, Stylish};

use crate::tokens_new::*;

const ERRORTXTSTYLE: Style = Style {
    foreground: Color::Red,
    background: Color::Empty,
    bold: true,
    dim: false,
    italic: true,
    underline: false,
};

#[derive(Debug, Clone, PartialEq)]
enum State {
    EmptyState,
    DefaultState,
    Number,
    Identifier,
    QuotedString,
    DoubleState,
    Comment,
}

pub struct Lexer<'a> {
    code: Peekable<Chars<'a>>,
    tokens: Vec<Token>,
    state: State,
    buffer_st: usize,
    buffer_ed: usize,
    line: usize,
    column: usize,
    source: &'a str,
}

impl<'a> Lexer<'a> {
    #[allow(dead_code)]
    pub fn new(code: &'a str) -> Self {
        Lexer {
            code: code.chars().peekable(),
            tokens: Vec::new(),
            state: State::EmptyState,
            buffer_st: 0,
            buffer_ed: 0,
            line: 1,
            column: 1,
            source: code,
        }
    }
    #[allow(dead_code)]
    pub fn run(&mut self) {
        while let Some(c) = self.next_char() {
            let peeked_char = match self.peek_char() {
                Ok(ch) => ch,
                Err(_) => '\0',  // Default value in case of error
            };

            // println!("---------------------------------");
            // println!("Current Char: {}", c);
            // println!("Current State: {:?}", self.state);
            // println!("Current Buffer: {}", self.source[self.buffer_st..self.buffer_ed].to_string());
            // println!("Current Buffer start: {}", self.buffer_st);
            // println!("Current Buffer end: {}", self.buffer_ed);
            if self.state == State::DoubleState {
                self.buffer_st = self.buffer_ed;
                self.state = State::EmptyState;
                continue;
            }

            // Handling Comment State
            if self.state == State::Comment {
                if c == '\n' {
                    self.state = State::EmptyState;
                    self.buffer_st = self.buffer_ed;
                }
                continue;
            }

            // Handling Whitespace
            if c.is_whitespace() && self.state != State::QuotedString {
                self.buffer_st = self.buffer_ed;
                self.state = State::EmptyState;
                continue;
            }

            // Check if the buffer is empty and the current character when is empty
            if self.buffer_ed == self.buffer_st + c.len_utf8() {
                // check if is a double symbol
                if peeked_char != '\0' {
                    let peeked_index = self.buffer_ed + peeked_char.len_utf8();
                    let double_symbol_str = &self.source[self.buffer_st..peeked_index];
                    let double_symbol =
                        symbol::double_symbol_to_token(double_symbol_str, self.line, self.column);
                    if let Ok(double_symbol) = double_symbol {
                        if double_symbol == Token::Symbol(Symbol::Comment) {
                            self.state = State::Comment;
                            continue;
                        }
                        self.insert_token(double_symbol);
                        self.state = State::DoubleState;
                        continue;
                    }
                }

                // Check for single symbols
                let symbol = symbol::symbol_to_token(c, self.line, self.column);
                if let Ok(symbol) = symbol {
                    self.insert_token(symbol);
                    self.buffer_st = self.buffer_ed;
                    continue;
                }

                // Handling operators
                let operator = symbol::operator_to_token(c, self.line, self.column);
                if let Ok(operator) = operator {
                    self.insert_token(operator);
                    self.buffer_st = self.buffer_ed;
                    continue;
                }

                self.state = State::DefaultState;
            }

            // Handling numbers
            let first_char: char = self.source[self.buffer_st..self.buffer_ed]
                .chars()
                .next()
                .unwrap();
            if self.state == State::DefaultState && (first_char == '-' || first_char.is_numeric()) {
                self.state = State::Number;
            }
            if self.state == State::Number && !peeked_char.is_numeric() {
                let number = value::number_to_token(
                    &self.source[self.buffer_st..self.buffer_ed],
                    self.line,
                    self.column,
                );
                match number {
                    Ok(number) => {
                        self.insert_token(number);
                        self.buffer_st = self.buffer_ed;
                    }
                    Err(error) => self.report_error(error),
                }

                self.state = State::EmptyState;
                continue;
            }

            // Handling quoted strings
            if self.state == State::DefaultState && identifier::is_quote(first_char) {
                self.state = State::QuotedString;
                continue;
            } else if self.state == State::QuotedString && !identifier::is_quote(c) {
                continue;
            } else if self.state == State::QuotedString && identifier::is_quote(c) {
                let mut string = &self.source[self.buffer_st..self.buffer_ed];
                string = string.trim_matches('"');
                self.insert_token(Token::TypeValue(TypeValue::QuotedString(
                    string.to_string(),
                )));
                self.buffer_st = self.buffer_ed;
                self.state = State::EmptyState;
                continue;
            }

            // check if is a identifier, statement, or symbol
            if self.state == State::DefaultState && identifier::is_first_identifierable(first_char)
            {
                self.state = State::Identifier;
            }
            if self.state == State::Identifier && !identifier::is_identifierable(peeked_char) {
                let string = &self.source[self.buffer_st..self.buffer_ed];
                let statement = identifier::statement_to_token(string, self.line, self.column);
                if let Ok(statement) = statement {
                    self.insert_token(statement);
                    self.buffer_st = self.buffer_ed;
                    self.state = State::EmptyState;
                    continue;
                }
                let type_name = identifier::type_name_to_token(string, self.line, self.column);
                if let Ok(type_name) = type_name {
                    self.insert_token(type_name);
                    self.buffer_st = self.buffer_ed;
                    self.state = State::EmptyState;
                    continue;
                }
                let identifier = Token::TypeValue(TypeValue::Identifier(string.to_string()));
                self.insert_token(identifier);
                self.buffer_st = self.buffer_ed;
                self.state = State::EmptyState;
                continue;
            }
        }
        if self.state == State::QuotedString {
            self.report_error(LexcialError {
                line: self.line,
                column: self.column,
                message: LexError::ExpectedQuote(),
            })
        }
    }

    fn next_char(&mut self) -> Option<char> {
        match self.code.next() {
            Some('\n') => {
                self.line += 1;
                self.column = '\n'.len_utf8();
                self.buffer_ed += '\n'.len_utf8(); // Advance buffer_end for the newline character
                Some('\n')
            }
            Some(ch) => {
                self.column += ch.len_utf8(); // Update column considering UTF-8 character length
                self.buffer_ed += ch.len_utf8(); // Advance buffer_end for the character
                Some(ch)
            }
            None => None,
        }
    }

    fn peek_char(&mut self) -> Result<char, ()> {
        self.code.peek().copied().ok_or(())
    }
    fn insert_token(&mut self, token: Token) {
        self.tokens.push(token);
    }

    fn report_error(&self, error: LexcialError) {
        let context_window = 10; // Number of characters to show around the error

        let start = self.buffer_st.saturating_sub(context_window);
        let end = std::cmp::min(self.buffer_ed + context_window, self.source.len());

        let context_snippet = &self.source[start..end];
        let error_location_marker = " ".repeat(self.column.saturating_sub(start) - 1) + "^";

        // Context and Error Information
        let errortxt = format!(
            "Context:\n{}\n{}\n--> Error at Line: {}, Column: {}: {}",
            context_snippet,
            error_location_marker,
            self.line,
            self.column,
            error.to_string().styled(ERRORTXTSTYLE)
        );

        // Suggestion for resolution (customize based on your error types)
        let suggestion = match error.message {
            LexError::InvalidCharacter(ch) => {
                format!(
                    "Suggestion: Unexpected character '{}'. Try removing or replacing it.",
                    ch
                )
            }
            LexError::InvalidTypeName(ch) => {
                format!("Suggestion: Unexpected type'{}'.", ch)
            }
            LexError::InvalidNumber(n) => {
                format!("Suggestion: Invalid number '{}'.", n)
            }
            LexError::InvalidIdentifier(i) => {
                format!("Suggestion: Invalid identifier '{}'.", i)
            }
            LexError::InvalidOperator(o) => {
                format!("Suggestion: Invalid operator '{}'.", o)
            }
            LexError::InvalidSymbol(s) => {
                format!("Suggestion: Invalid symbol '{}'.", s)
            }
            LexError::InvalidStatement(s) => {
                format!("Suggestion: Invalid statement '{}'.", s)
            }
            LexError::InvalidDoubleSymbol(s) => {
                format!("Suggestion: Invalid double symbol '{}'.", s)
            }
            LexError::ExpectedQuote() => {
                format!("Suggestion: Expected quote.")
            }
            _ => String::from("Suggestion: Check the syntax and correct the error."),
        };

        eprintln!("{}\n{}", errortxt, suggestion);
        std::process::exit(1);
    }

    pub fn get_tokens(&self) -> Vec<Token> {
        self.tokens.clone()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn line_counting() {
        let code = "fn main() -> Void \n{\nprintln(\"Hello, world!\");\n}";
        let mut lexer = Lexer::new(code);
        lexer.run();
        println!("{:?}", lexer.tokens);
        assert_eq!(lexer.line, 4);
    }
    #[test]
    fn column_counting() {
        let code = "fn main() -> Void\n{\nprintln(\"Hello, world!\");\n}";
        let mut lexer = Lexer::new(code);
        lexer.run();
        println!("{:?}", lexer.tokens);
        assert_eq!(lexer.column, 2);
    }
    #[test]
    fn lexing_numbers() {
        let code = "fn main() -> Void \n{\nlet:i32 a = 5;\nlet:i32 b = 0;\n}";
        let ans = vec![
            Token::Statement(Statement::Function),
            Token::TypeValue(TypeValue::Identifier("main".to_string())),
            Token::Symbol(Symbol::OpenParen),
            Token::Symbol(Symbol::CloseParen),
            Token::Symbol(Symbol::Arrow),
            Token::TypeName(TypeName::Void),
            Token::Symbol(Symbol::OpenBrace),
            Token::Statement(Statement::Let),
            Token::Symbol(Symbol::Colon),
            Token::TypeName(TypeName::I32),
            Token::TypeValue(TypeValue::Identifier("a".to_string())),
            Token::Assign(Assign::Assign),
            Token::TypeValue(TypeValue::Number(5.to_string())),
            Token::Symbol(Symbol::Semicolon),
            Token::Statement(Statement::Let),
            Token::Symbol(Symbol::Colon),
            Token::TypeName(TypeName::I32),
            Token::TypeValue(TypeValue::Identifier("b".to_string())),
            Token::Assign(Assign::Assign),
            Token::TypeValue(TypeValue::Number(0.to_string())),
            Token::Symbol(Symbol::Semicolon),
            Token::Symbol(Symbol::CloseBrace),
        ];
        let mut lexer = Lexer::new(code);
        lexer.run();
        println!("{:?}", lexer.tokens);
        assert_eq!(lexer.tokens, ans);
    }
    #[test]
    fn lexing_strings() {
        let code = " \"Hello, world!\" ";
        let ans = vec![Token::TypeValue(TypeValue::QuotedString(
            "Hello, world!".to_string(),
        ))];
        let mut lexer = Lexer::new(code);
        lexer.run();
        println!("{:?}", lexer.tokens);
        assert_eq!(lexer.tokens, ans);
    }
    #[test]
    fn lexing_comments() {
        let code = "public fn main() -> Void \n{\n//println(\"Hello, world!\");\nreturn;\n}";
        let ans = vec![
            Token::Statement(Statement::Public),
            Token::Statement(Statement::Function),
            Token::TypeValue(TypeValue::Identifier("main".to_string())),
            Token::Symbol(Symbol::OpenParen),
            Token::Symbol(Symbol::CloseParen),
            Token::Symbol(Symbol::Arrow),
            Token::TypeName(TypeName::Void),
            Token::Symbol(Symbol::OpenBrace),
            Token::Statement(Statement::Return),
            Token::Symbol(Symbol::Semicolon),
            Token::Symbol(Symbol::CloseBrace),
        ];
        let mut lexer = Lexer::new(code);
        lexer.run();
        println!("{:?}", lexer.tokens);
        assert_eq!(lexer.tokens, ans);
    }
    #[test]
    fn lexing_string_assign() {
        let code = "let:String a = \"Hello, world!\";";
        let ans = vec![
            Token::Statement(Statement::Let),
            Token::Symbol(Symbol::Colon),
            Token::TypeName(TypeName::QuotedString),
            Token::TypeValue(TypeValue::Identifier("a".to_string())),
            Token::Assign(Assign::Assign),
            Token::TypeValue(TypeValue::QuotedString("Hello, world!".to_string())),
            Token::Symbol(Symbol::Semicolon),
        ];
        let mut lexer = Lexer::new(code);
        lexer.run();
        println!("{:?}", lexer.tokens);
        assert_eq!(lexer.tokens, ans);
    }
    #[test]
    fn lexing_underbar_started_var() {
        let code = "let:i32 _a = 5;";
        let ans = vec![
            Token::Statement(Statement::Let),
            Token::Symbol(Symbol::Colon),
            Token::TypeName(TypeName::I32),
            Token::TypeValue(TypeValue::Identifier("_a".to_string())),
            Token::Assign(Assign::Assign),
            Token::TypeValue(TypeValue::Number(5.to_string())),
            Token::Symbol(Symbol::Semicolon),
        ];
        let mut lexer = Lexer::new(code);
        lexer.run();
        println!("{:?}", lexer.tokens);
        assert_eq!(lexer.tokens, ans);
    }
    /*#[test]
    fn lexing_negative_number_assign() {
    let code = "let:i32 a = -5;";
    let ans = vec![
    Token::Statement(Statement::Let),
    Token::Symbol(Symbol::Colon),
    Token::TypeName(TypeName::I32),
    Token::TypeValue(TypeValue::Identifier("a".to_string())),
    Token::Assign(Assign::Assign),
    Token::TypeValue(TypeValue::Number("-5".to_string())),
    Token::Symbol(Symbol::Semicolon),
    ];
    let mut lexer = Lexer::new(code);
    lexer.run();
    println!("{:?}", lexer.tokens);
    assert_eq!(lexer.tokens, ans);
    }*/
    #[test]
    fn lexing_nested_expression() {
        let code = "let:i32 a = ((5 + a) /2)+2;";
        let ans = vec![
            Token::Statement(Statement::Let),
            Token::Symbol(Symbol::Colon),
            Token::TypeName(TypeName::I32),
            Token::TypeValue(TypeValue::Identifier("a".to_string())),
            Token::Assign(Assign::Assign),
            Token::Symbol(Symbol::OpenParen),
            Token::Symbol(Symbol::OpenParen),
            Token::TypeValue(TypeValue::Number(5.to_string())),
            Token::Operator(Operator::Add),
            Token::TypeValue(TypeValue::Identifier("a".to_string())),
            Token::Symbol(Symbol::CloseParen),
            Token::Operator(Operator::Divide),
            Token::TypeValue(TypeValue::Number(2.to_string())),
            Token::Symbol(Symbol::CloseParen),
            Token::Operator(Operator::Add),
            Token::TypeValue(TypeValue::Number(2.to_string())),
            Token::Symbol(Symbol::Semicolon),
        ];
        let mut lexer = Lexer::new(code);
        lexer.run();
        println!("{:?}", lexer.tokens);
        assert_eq!(lexer.tokens, ans);
    }
    #[test]
    fn lexing_complex() {
        let code = "fn main() -> Void \n{\nlet:i32 a = 5;\nlet:i32 b = 0;\nprintln(\"Hello, world!\");\nreturn;\n}";
        let ans = vec![
            Token::Statement(Statement::Function),
            Token::TypeValue(TypeValue::Identifier("main".to_string())),
            Token::Symbol(Symbol::OpenParen),
            Token::Symbol(Symbol::CloseParen),
            Token::Symbol(Symbol::Arrow),
            Token::TypeName(TypeName::Void),
            Token::Symbol(Symbol::OpenBrace),
            Token::Statement(Statement::Let),
            Token::Symbol(Symbol::Colon),
            Token::TypeName(TypeName::I32),
            Token::TypeValue(TypeValue::Identifier("a".to_string())),
            Token::Assign(Assign::Assign),
            Token::TypeValue(TypeValue::Number(5.to_string())),
            Token::Symbol(Symbol::Semicolon),
            Token::Statement(Statement::Let),
            Token::Symbol(Symbol::Colon),
            Token::TypeName(TypeName::I32),
            Token::TypeValue(TypeValue::Identifier("b".to_string())),
            Token::Assign(Assign::Assign),
            Token::TypeValue(TypeValue::Number(0.to_string())),
            Token::Symbol(Symbol::Semicolon),
            Token::Statement(Statement::Println),
            Token::Symbol(Symbol::OpenParen),
            Token::TypeValue(TypeValue::QuotedString("Hello, world!".to_string())),
            Token::Symbol(Symbol::CloseParen),
            Token::Symbol(Symbol::Semicolon),
            Token::Statement(Statement::Return),
            Token::Symbol(Symbol::Semicolon),
            Token::Symbol(Symbol::CloseBrace),
        ];
        let mut lexer = Lexer::new(code);
        lexer.run();
        println!("{:?}", lexer.tokens);
        assert_eq!(lexer.tokens, ans);
    }
}
