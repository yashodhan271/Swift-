use clap::{App, Arg};
use std::fs;
use std::path::Path;

mod compiler;

fn main() {
    // Setup logging
    env_logger::init();

    // Parse command line arguments
    let matches = App::new("Swift++ Compiler")
        .version("0.1.0")
        .author("Your Name")
        .about("Compiler for the Swift++ programming language")
        .arg(
            Arg::with_name("INPUT")
                .help("Input source file")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("output")
                .short('o')
                .long("output")
                .value_name("FILE")
                .help("Output file")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("verbose")
                .short('v')
                .long("verbose")
                .help("Enable verbose output"),
        )
        .get_matches();

    // Get input file
    let input_path = matches.value_of("INPUT").unwrap();
    let output_path = matches
        .value_of("output")
        .unwrap_or(&format!("{}.o", input_path));

    // Read source file
    let source = match fs::read_to_string(input_path) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Error reading source file: {}", e);
            std::process::exit(1);
        }
    };

    // Create and run compiler
    let compiler = compiler::Compiler::new(source, output_path.to_string());
    match compiler.compile() {
        Ok(_) => {
            println!("Compilation successful!");
            println!("Output written to: {}", output_path);
        }
        Err(e) => {
            eprintln!("Compilation error: {}", e);
            std::process::exit(1);
        }
    }
}
