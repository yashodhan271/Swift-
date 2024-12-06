use std::str::Chars;
use std::iter::Peekable;

#[derive(Debug, PartialEq, Clone)]
pub enum TokenType {
    // Keywords
    Let,
    Fn,
    Return,
    If,
    Else,
    While,
    For,
    Struct,
    Enum,
    Match,
    Async,
    Await,
    Move,
    Own,
    Ref,
    
    // Literals
    Integer(i64),
    Float(f64),
    String(String),
    Identifier(String),
    
    // Operators
    Plus,
    Minus,
    Multiply,
    Divide,
    Assign,
    Equal,
    NotEqual,
    Greater,
    Less,
    
    // Delimiters
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    Semicolon,
    Colon,
    Comma,
    Arrow,
    
    EOF,
}

#[derive(Debug)]
pub struct Token {
    pub token_type: TokenType,
    pub line: usize,
    pub column: usize,
}

pub struct Lexer<'a> {
    input: Peekable<Chars<'a>>,
    line: usize,
    column: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Lexer {
            input: input.chars().peekable(),
            line: 1,
            column: 1,
        }
    }
    
    fn advance(&mut self) -> Option<char> {
        let c = self.input.next();
        if let Some(ch) = c {
            if ch == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
        }
        c
    }
    
    fn peek(&mut self) -> Option<&char> {
        self.input.peek()
    }
    
    fn skip_whitespace(&mut self) {
        while let Some(&c) = self.peek() {
            if !c.is_whitespace() {
                break;
            }
            self.advance();
        }
    }
    
    fn read_identifier(&mut self, first_char: char) -> String {
        let mut identifier = String::new();
        identifier.push(first_char);
        
        while let Some(&c) = self.peek() {
            if !c.is_alphanumeric() && c != '_' {
                break;
            }
            identifier.push(self.advance().unwrap());
        }
        
        identifier
    }
    
    fn read_number(&mut self, first_char: char) -> TokenType {
        let mut number = String::new();
        number.push(first_char);
        let mut is_float = false;
        
        while let Some(&c) = self.peek() {
            if !c.is_digit(10) && c != '.' {
                break;
            }
            if c == '.' {
                if is_float {
                    panic!("Invalid number format");
                }
                is_float = true;
            }
            number.push(self.advance().unwrap());
        }
        
        if is_float {
            TokenType::Float(number.parse().unwrap())
        } else {
            TokenType::Integer(number.parse().unwrap())
        }
    }
    
    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace();
        
        let current_line = self.line;
        let current_column = self.column;
        
        let token_type = match self.advance() {
            None => TokenType::EOF,
            Some(c) => match c {
                '(' => TokenType::LeftParen,
                ')' => TokenType::RightParen,
                '{' => TokenType::LeftBrace,
                '}' => TokenType::RightBrace,
                '[' => TokenType::LeftBracket,
                ']' => TokenType::RightBracket,
                ';' => TokenType::Semicolon,
                ':' => TokenType::Colon,
                ',' => TokenType::Comma,
                '+' => TokenType::Plus,
                '-' => {
                    if let Some(&'>') = self.peek() {
                        self.advance();
                        TokenType::Arrow
                    } else {
                        TokenType::Minus
                    }
                },
                '*' => TokenType::Multiply,
                '/' => TokenType::Divide,
                '=' => {
                    if let Some(&'=') = self.peek() {
                        self.advance();
                        TokenType::Equal
                    } else {
                        TokenType::Assign
                    }
                },
                c if c.is_alphabetic() || c == '_' => {
                    let ident = self.read_identifier(c);
                    match ident.as_str() {
                        "let" => TokenType::Let,
                        "fn" => TokenType::Fn,
                        "return" => TokenType::Return,
                        "if" => TokenType::If,
                        "else" => TokenType::Else,
                        "while" => TokenType::While,
                        "for" => TokenType::For,
                        "struct" => TokenType::Struct,
                        "enum" => TokenType::Enum,
                        "match" => TokenType::Match,
                        "async" => TokenType::Async,
                        "await" => TokenType::Await,
                        "move" => TokenType::Move,
                        "own" => TokenType::Own,
                        "ref" => TokenType::Ref,
                        _ => TokenType::Identifier(ident),
                    }
                },
                c if c.is_digit(10) => self.read_number(c),
                _ => panic!("Unexpected character: {}", c),
            },
        };
        
        Token {
            token_type,
            line: current_line,
            column: current_column,
        }
    }
}
