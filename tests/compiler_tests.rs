use swiftpp::compiler::{Compiler, lexer, parser, analyzer};
use pretty_assertions::assert_eq;

#[test]
fn test_lexer() {
    let source = r#"
        fn main() -> i32 {
            let x: i32 = 42;
            return x;
        }
    "#;

    let mut lexer = lexer::Lexer::new(source);
    let tokens: Vec<_> = std::iter::from_fn(|| {
        let token = lexer.next_token();
        if token.token_type == lexer::TokenType::EOF {
            None
        } else {
            Some(token)
        }
    }).collect();

    assert_eq!(tokens.len(), 12); // fn, main, (), ->, i32, {, let, x, :, i32, =, 42, ;, return, x, ;, }
}

#[test]
fn test_parser() {
    let source = r#"
        fn add(a: i32, b: i32) -> i32 {
            return a + b;
        }
    "#;

    let mut lexer = lexer::Lexer::new(source);
    let tokens: Vec<_> = std::iter::from_fn(|| {
        let token = lexer.next_token();
        if token.token_type == lexer::TokenType::EOF {
            None
        } else {
            Some(token)
        }
    }).collect();

    let mut parser = parser::Parser::new(tokens);
    let ast = parser.parse().expect("Failed to parse");

    match &ast.statements[0] {
        parser::ast::Statement::Function { name, params, return_type, .. } => {
            assert_eq!(name, "add");
            assert_eq!(params.len(), 2);
            assert!(return_type.is_some());
        }
        _ => panic!("Expected function definition"),
    }
}

#[test]
fn test_type_checker() {
    let source = r#"
        fn main() -> i32 {
            let x: i32 = 42;
            let y: f64 = 3.14;
            let z: i32 = x + y; // Type error
            return x;
        }
    "#;

    let mut lexer = lexer::Lexer::new(source);
    let tokens: Vec<_> = std::iter::from_fn(|| {
        let token = lexer.next_token();
        if token.token_type == lexer::TokenType::EOF {
            None
        } else {
            Some(token)
        }
    }).collect();

    let mut parser = parser::Parser::new(tokens);
    let ast = parser.parse().expect("Failed to parse");

    let mut analyzer = analyzer::SemanticAnalyzer::new();
    let result = analyzer.analyze(&ast);
    assert!(result.is_err()); // Should fail due to type mismatch
}

#[test]
fn test_optimization() {
    let source = r#"
        fn factorial(n: i32) -> i32 {
            if n <= 1 {
                return 1;
            }
            return n * factorial(n - 1);
        }
    "#;

    let compiler = Compiler::new(source.to_string(), "test.o".to_string());
    let result = compiler.compile();
    assert!(result.is_ok());
}

// Integration tests
#[test]
fn test_end_to_end() {
    let source = r#"
        fn fibonacci(n: i32) -> i32 {
            if n <= 1 {
                return n;
            }
            return fibonacci(n - 1) + fibonacci(n - 2);
        }

        fn main() -> i32 {
            return fibonacci(10);
        }
    "#;

    let compiler = Compiler::new(source.to_string(), "test_fib.o".to_string());
    assert!(compiler.compile().is_ok());
}
