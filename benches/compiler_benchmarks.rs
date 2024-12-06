use criterion::{black_box, criterion_group, criterion_main, Criterion};
use swiftpp::compiler::{Compiler, lexer, parser, analyzer};

fn benchmark_lexer(c: &mut Criterion) {
    let source = include_str!("../examples/hello.spp");
    
    c.bench_function("lexer", |b| {
        b.iter(|| {
            let mut lexer = lexer::Lexer::new(black_box(source));
            while lexer.next_token().token_type != lexer::TokenType::EOF {}
        })
    });
}

fn benchmark_parser(c: &mut Criterion) {
    let source = include_str!("../examples/hello.spp");
    let mut lexer = lexer::Lexer::new(source);
    let tokens: Vec<_> = std::iter::from_fn(|| {
        let token = lexer.next_token();
        if token.token_type == lexer::TokenType::EOF {
            None
        } else {
            Some(token)
        }
    }).collect();
    
    c.bench_function("parser", |b| {
        b.iter(|| {
            let mut parser = parser::Parser::new(black_box(tokens.clone()));
            parser.parse().unwrap()
        })
    });
}

fn benchmark_type_checker(c: &mut Criterion) {
    let source = include_str!("../examples/hello.spp");
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
    let ast = parser.parse().unwrap();
    
    c.bench_function("type_checker", |b| {
        b.iter(|| {
            let mut analyzer = analyzer::SemanticAnalyzer::new();
            analyzer.analyze(black_box(&ast)).unwrap()
        })
    });
}

fn benchmark_codegen(c: &mut Criterion) {
    let source = include_str!("../examples/hello.spp");
    
    c.bench_function("codegen", |b| {
        b.iter(|| {
            let compiler = Compiler::new(black_box(source.to_string()), "bench.o".to_string());
            compiler.compile().unwrap()
        })
    });
}

fn benchmark_optimization(c: &mut Criterion) {
    let source = r#"
        fn factorial(n: i32) -> i32 {
            if n <= 1 {
                return 1;
            }
            return n * factorial(n - 1);
        }
    "#;
    
    c.bench_function("optimization", |b| {
        b.iter(|| {
            let compiler = Compiler::new(black_box(source.to_string()), "bench_opt.o".to_string());
            compiler.compile().unwrap()
        })
    });
}

criterion_group!(
    benches,
    benchmark_lexer,
    benchmark_parser,
    benchmark_type_checker,
    benchmark_codegen,
    benchmark_optimization
);
criterion_main!(benches);
