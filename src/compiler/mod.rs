pub mod lexer;
pub mod ast;
pub mod parser;
pub mod analyzer;
pub mod codegen;

use std::path::Path;
use std::fs;

pub struct Compiler {
    pub source: String,
    pub output_path: String,
}

impl Compiler {
    pub fn new(source: String, output_path: String) -> Self {
        Compiler {
            source,
            output_path,
        }
    }

    pub fn compile(&self) -> Result<(), String> {
        // Step 1: Lexical Analysis
        let mut lexer = lexer::Lexer::new(&self.source);
        let mut tokens = Vec::new();
        loop {
            let token = lexer.next_token();
            if token.token_type == lexer::TokenType::EOF {
                break;
            }
            tokens.push(token);
        }

        // Step 2: Parsing
        let mut parser = parser::Parser::new(tokens);
        let ast = parser.parse()?;

        // Step 3: Semantic Analysis
        let mut analyzer = analyzer::SemanticAnalyzer::new();
        analyzer.analyze(&ast)?;

        // Step 4: Code Generation
        let mut codegen = codegen::CodeGenerator::new("swiftpp_module");
        codegen.generate(&ast)?;

        Ok(())
    }
}
