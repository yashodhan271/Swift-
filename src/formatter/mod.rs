use crate::compiler::{ast, lexer, parser};
use std::fmt::Write;

pub struct Formatter {
    indent_level: usize,
    indent_str: String,
    line_width: usize,
    output: String,
}

impl Formatter {
    pub fn new() -> Self {
        Formatter {
            indent_level: 0,
            indent_str: "    ".to_string(), // 4 spaces
            line_width: 100,
            output: String::new(),
        }
    }

    pub fn format(&mut self, source: &str) -> Result<String, String> {
        // Parse the source code
        let mut lexer = lexer::Lexer::new(source);
        let tokens: Vec<_> = std::iter::from_fn(|| {
            let token = lexer.next_token();
            if token.token_type == lexer::TokenType::EOF {
                None
            } else {
                Some(token)
            }
        })
        .collect();

        let mut parser = parser::Parser::new(tokens);
        let ast = parser.parse()?;

        // Reset formatter state
        self.indent_level = 0;
        self.output.clear();

        // Format AST
        self.format_ast(&ast)?;

        Ok(self.output.clone())
    }

    fn format_ast(&mut self, ast: &ast::Program) -> Result<(), String> {
        for item in &ast.items {
            self.format_item(item)?;
            self.newline()?;
        }
        Ok(())
    }

    fn format_item(&mut self, item: &ast::Item) -> Result<(), String> {
        match item {
            ast::Item::Function(func) => self.format_function(func),
            ast::Item::Struct(struct_def) => self.format_struct(struct_def),
            ast::Item::Enum(enum_def) => self.format_enum(enum_def),
            // Add more item types as needed
        }
    }

    fn format_function(&mut self, func: &ast::Function) -> Result<(), String> {
        // Write function signature
        write!(self.output, "fn {}(", func.name).map_err(|e| e.to_string())?;
        
        // Format parameters
        for (i, param) in func.parameters.iter().enumerate() {
            if i > 0 {
                write!(self.output, ", ").map_err(|e| e.to_string())?;
            }
            write!(self.output, "{}: {}", param.name, param.type_name)
                .map_err(|e| e.to_string())?;
        }
        
        write!(self.output, ") -> {} ", func.return_type).map_err(|e| e.to_string())?;
        
        // Format function body
        self.format_block(&func.body)
    }

    fn format_struct(&mut self, struct_def: &ast::Struct) -> Result<(), String> {
        writeln!(self.output, "struct {} {{", struct_def.name).map_err(|e| e.to_string())?;
        self.indent_level += 1;

        for field in &struct_def.fields {
            self.indent()?;
            writeln!(self.output, "{}: {},", field.name, field.type_name)
                .map_err(|e| e.to_string())?;
        }

        self.indent_level -= 1;
        self.indent()?;
        writeln!(self.output, "}}").map_err(|e| e.to_string())
    }

    fn format_enum(&mut self, enum_def: &ast::Enum) -> Result<(), String> {
        writeln!(self.output, "enum {} {{", enum_def.name).map_err(|e| e.to_string())?;
        self.indent_level += 1;

        for variant in &enum_def.variants {
            self.indent()?;
            match &variant.data {
                ast::EnumVariantData::Unit => {
                    writeln!(self.output, "{},", variant.name).map_err(|e| e.to_string())?;
                }
                ast::EnumVariantData::Tuple(types) => {
                    write!(self.output, "{}(", variant.name).map_err(|e| e.to_string())?;
                    for (i, ty) in types.iter().enumerate() {
                        if i > 0 {
                            write!(self.output, ", ").map_err(|e| e.to_string())?;
                        }
                        write!(self.output, "{}", ty).map_err(|e| e.to_string())?;
                    }
                    writeln!(self.output, "),").map_err(|e| e.to_string())?;
                }
                ast::EnumVariantData::Struct(fields) => {
                    writeln!(self.output, "{} {{", variant.name).map_err(|e| e.to_string())?;
                    self.indent_level += 1;
                    for field in fields {
                        self.indent()?;
                        writeln!(self.output, "{}: {},", field.name, field.type_name)
                            .map_err(|e| e.to_string())?;
                    }
                    self.indent_level -= 1;
                    self.indent()?;
                    writeln!(self.output, "}},").map_err(|e| e.to_string())?;
                }
            }
        }

        self.indent_level -= 1;
        self.indent()?;
        writeln!(self.output, "}}").map_err(|e| e.to_string())
    }

    fn format_block(&mut self, block: &ast::Block) -> Result<(), String> {
        writeln!(self.output, "{{").map_err(|e| e.to_string())?;
        self.indent_level += 1;

        for stmt in &block.statements {
            self.indent()?;
            self.format_statement(stmt)?;
        }

        self.indent_level -= 1;
        self.indent()?;
        writeln!(self.output, "}}").map_err(|e| e.to_string())
    }

    fn format_statement(&mut self, stmt: &ast::Statement) -> Result<(), String> {
        match stmt {
            ast::Statement::Let(var) => {
                writeln!(
                    self.output,
                    "let {}: {} = {};",
                    var.name,
                    var.type_name,
                    self.format_expression(&var.initializer)?
                )
                .map_err(|e| e.to_string())
            }
            ast::Statement::Return(expr) => {
                if let Some(e) = expr {
                    writeln!(
                        self.output,
                        "return {};",
                        self.format_expression(e)?
                    )
                    .map_err(|e| e.to_string())
                } else {
                    writeln!(self.output, "return;").map_err(|e| e.to_string())
                }
            }
            ast::Statement::Expression(expr) => {
                writeln!(
                    self.output,
                    "{};",
                    self.format_expression(expr)?
                )
                .map_err(|e| e.to_string())
            }
            // Add more statement types
        }
    }

    fn format_expression(&mut self, expr: &ast::Expression) -> Result<String, String> {
        match expr {
            ast::Expression::Literal(lit) => Ok(lit.to_string()),
            ast::Expression::Identifier(name) => Ok(name.clone()),
            ast::Expression::BinaryOp { left, op, right } => {
                Ok(format!(
                    "{} {} {}",
                    self.format_expression(left)?,
                    op,
                    self.format_expression(right)?
                ))
            }
            ast::Expression::Call { function, args } => {
                let mut result = self.format_expression(function)?;
                result.push('(');
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        result.push_str(", ");
                    }
                    result.push_str(&self.format_expression(arg)?);
                }
                result.push(')');
                Ok(result)
            }
            // Add more expression types
        }
    }

    fn indent(&mut self) -> Result<(), String> {
        for _ in 0..self.indent_level {
            self.output.push_str(&self.indent_str);
        }
        Ok(())
    }

    fn newline(&mut self) -> Result<(), String> {
        writeln!(self.output).map_err(|e| e.to_string())
    }
}
