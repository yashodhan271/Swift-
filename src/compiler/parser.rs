use super::lexer::{Token, TokenType};
use super::ast::*;

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser {
            tokens,
            current: 0,
        }
    }

    pub fn parse(&mut self) -> Result<Program, String> {
        let mut statements = Vec::new();
        while !self.is_at_end() {
            statements.push(self.parse_statement()?);
        }
        Ok(Program { statements })
    }

    fn parse_statement(&mut self) -> Result<Statement, String> {
        match &self.peek().token_type {
            TokenType::Let => self.parse_let_statement(),
            TokenType::Fn => self.parse_function(),
            TokenType::Return => self.parse_return_statement(),
            TokenType::If => self.parse_if_statement(),
            TokenType::While => self.parse_while_statement(),
            TokenType::Struct => self.parse_struct_declaration(),
            _ => Ok(Statement::Expression(self.parse_expression()?)),
        }
    }

    fn parse_let_statement(&mut self) -> Result<Statement, String> {
        self.consume(TokenType::Let, "Expected 'let'")?;
        let name = match &self.consume_any()?.token_type {
            TokenType::Identifier(name) => name.clone(),
            _ => return Err("Expected identifier".to_string()),
        };

        let type_annotation = if self.match_token(TokenType::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };

        self.consume(TokenType::Assign, "Expected '='")?;
        let initializer = self.parse_expression()?;
        self.consume(TokenType::Semicolon, "Expected ';'")?;

        Ok(Statement::Let {
            name,
            type_annotation,
            initializer,
        })
    }

    fn parse_expression(&mut self) -> Result<Expression, String> {
        self.parse_binary_expression()
    }

    fn parse_binary_expression(&mut self) -> Result<Expression, String> {
        let mut expr = self.parse_primary()?;

        while let Some(op) = self.match_binary_operator() {
            let right = self.parse_primary()?;
            expr = Expression::Binary {
                left: Box::new(expr),
                operator: op,
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expression, String> {
        match &self.peek().token_type {
            TokenType::Integer(n) => {
                self.advance();
                Ok(Expression::Integer(*n))
            }
            TokenType::Float(n) => {
                self.advance();
                Ok(Expression::Float(*n))
            }
            TokenType::Identifier(name) => {
                self.advance();
                if self.match_token(TokenType::LeftParen) {
                    self.parse_call(name.clone())
                } else {
                    Ok(Expression::Identifier(name.clone()))
                }
            }
            _ => Err("Expected expression".to_string()),
        }
    }

    fn parse_type(&mut self) -> Result<Type, String> {
        match &self.consume_any()?.token_type {
            TokenType::Identifier(name) => match name.as_str() {
                "i32" => Ok(Type::Int),
                "f64" => Ok(Type::Float),
                "string" => Ok(Type::String),
                "bool" => Ok(Type::Bool),
                _ => Ok(Type::Custom(name.clone())),
            },
            _ => Err("Expected type".to_string()),
        }
    }

    // Helper methods
    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn is_at_end(&self) -> bool {
        self.peek().token_type == TokenType::EOF
    }

    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous()
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.current - 1]
    }

    fn consume(&mut self, token_type: TokenType, message: &str) -> Result<&Token, String> {
        if self.check(&token_type) {
            Ok(self.advance())
        } else {
            Err(message.to_string())
        }
    }

    fn consume_any(&mut self) -> Result<&Token, String> {
        if !self.is_at_end() {
            Ok(self.advance())
        } else {
            Err("Unexpected end of input".to_string())
        }
    }

    fn check(&self, token_type: &TokenType) -> bool {
        if self.is_at_end() {
            false
        } else {
            &self.peek().token_type == token_type
        }
    }

    fn match_token(&mut self, token_type: TokenType) -> bool {
        if self.check(&token_type) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn match_binary_operator(&mut self) -> Option<BinaryOp> {
        match &self.peek().token_type {
            TokenType::Plus => {
                self.advance();
                Some(BinaryOp::Add)
            }
            TokenType::Minus => {
                self.advance();
                Some(BinaryOp::Subtract)
            }
            TokenType::Multiply => {
                self.advance();
                Some(BinaryOp::Multiply)
            }
            TokenType::Divide => {
                self.advance();
                Some(BinaryOp::Divide)
            }
            _ => None,
        }
    }
}
