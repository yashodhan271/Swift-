use std::path::PathBuf;
use std::sync::Arc;
use crate::compiler::ast;
use crate::compiler::parser;
use crate::compiler::lexer;
use crate::debugger::memory::MemoryInspector;

#[derive(Debug, Clone)]
pub enum BreakpointType {
    Normal,
    Conditional(String),
    LogPoint(String),
    Counter { count: usize, hit_count: usize },
}

#[derive(Debug)]
pub struct Breakpoint {
    pub id: usize,
    pub address: usize,
    pub original_byte: u8,
    pub enabled: bool,
    pub line: u32,
    pub file: PathBuf,
    pub breakpoint_type: BreakpointType,
}

pub struct BreakpointManager {
    breakpoints: Vec<Breakpoint>,
    memory_inspector: Arc<MemoryInspector>,
    next_id: usize,
}

impl BreakpointManager {
    pub fn new(memory_inspector: Arc<MemoryInspector>) -> Self {
        BreakpointManager {
            breakpoints: Vec::new(),
            memory_inspector,
            next_id: 1,
        }
    }

    pub fn add_breakpoint(
        &mut self,
        address: usize,
        line: u32,
        file: PathBuf,
        breakpoint_type: BreakpointType,
    ) -> Result<usize, String> {
        // Read original byte
        let original_byte = self.memory_inspector.read_memory::<u8>(address)
            .map_err(|e| format!("Failed to read memory: {:?}", e))?;

        // Write int3 instruction (0xCC)
        self.memory_inspector.write_memory(address, 0xCCu8)
            .map_err(|e| format!("Failed to write breakpoint: {:?}", e))?;

        let id = self.next_id;
        self.next_id += 1;

        self.breakpoints.push(Breakpoint {
            id,
            address,
            original_byte,
            enabled: true,
            line,
            file,
            breakpoint_type,
        });

        Ok(id)
    }

    pub fn remove_breakpoint(&mut self, id: usize) -> Result<(), String> {
        if let Some(index) = self.breakpoints.iter().position(|bp| bp.id == id) {
            let bp = &self.breakpoints[index];
            // Restore original byte
            self.memory_inspector.write_memory(bp.address, bp.original_byte)
                .map_err(|e| format!("Failed to restore original byte: {:?}", e))?;
            self.breakpoints.remove(index);
            Ok(())
        } else {
            Err("Breakpoint not found".to_string())
        }
    }

    pub fn evaluate_condition(&self, condition: &str, memory_inspector: &MemoryInspector) -> Result<bool, String> {
        // Parse condition into AST
        let mut lexer = lexer::Lexer::new(condition);
        let tokens: Vec<_> = std::iter::from_fn(|| {
            let token = lexer.next_token();
            if token.token_type == lexer::TokenType::EOF {
                None
            } else {
                Some(token)
            }
        }).collect();

        let mut parser = parser::Parser::new(tokens);
        let expr = parser.parse_expression()
            .map_err(|e| format!("Failed to parse condition: {}", e))?;

        // Evaluate expression
        self.evaluate_expression(&expr, memory_inspector)
    }

    fn evaluate_expression(&self, expr: &ast::Expression, memory_inspector: &MemoryInspector) -> Result<bool, String> {
        match expr {
            ast::Expression::BinaryOp { left, op, right } => {
                let left_val = self.evaluate_expression(left, memory_inspector)?;
                let right_val = self.evaluate_expression(right, memory_inspector)?;
                match op.as_str() {
                    "==" => Ok(left_val == right_val),
                    "!=" => Ok(left_val != right_val),
                    "<" => Ok(left_val < right_val),
                    "<=" => Ok(left_val <= right_val),
                    ">" => Ok(left_val > right_val),
                    ">=" => Ok(left_val >= right_val),
                    "&&" => Ok(left_val && right_val),
                    "||" => Ok(left_val || right_val),
                    _ => Err(format!("Unsupported operator: {}", op)),
                }
            }
            ast::Expression::Identifier(name) => {
                // Look up variable value in debug info
                // This is a simplified example - real implementation would need to handle
                // variable scope and types properly
                Ok(false)
            }
            ast::Expression::Literal(value) => {
                match value {
                    ast::Literal::Bool(b) => Ok(*b),
                    ast::Literal::Int(i) => Ok(*i != 0),
                    _ => Err("Unsupported literal type in condition".to_string()),
                }
            }
            _ => Err("Unsupported expression type in condition".to_string()),
        }
    }

    pub fn format_log_message(&self, message: &str, memory_inspector: &MemoryInspector) -> Result<String, String> {
        let mut result = String::new();
        let mut current = 0;

        while let Some(start) = message[current..].find('{') {
            result.push_str(&message[current..current + start]);
            current += start;

            if let Some(end) = message[current..].find('}') {
                let expr = &message[current + 1..current + end];
                let value = self.evaluate_expression_for_log(expr, memory_inspector)?;
                result.push_str(&value);
                current += end + 1;
            } else {
                return Err("Unterminated placeholder in log message".to_string());
            }
        }

        result.push_str(&message[current..]);
        Ok(result)
    }

    fn evaluate_expression_for_log(&self, expr: &str, memory_inspector: &MemoryInspector) -> Result<String, String> {
        // Parse and evaluate expression for logging
        // This is a simplified implementation
        Ok(expr.to_string())
    }

    pub fn handle_breakpoint(&mut self, address: usize) -> Option<&Breakpoint> {
        self.breakpoints.iter().find(|bp| bp.address == address && bp.enabled)
    }

    pub fn should_break(&self, breakpoint: &Breakpoint, memory_inspector: &MemoryInspector) -> Result<bool, String> {
        match &breakpoint.breakpoint_type {
            BreakpointType::Normal => Ok(true),
            BreakpointType::Conditional(condition) => {
                self.evaluate_condition(condition, memory_inspector)
            }
            BreakpointType::LogPoint(_) => Ok(false),
            BreakpointType::Counter { count, hit_count } => {
                Ok(*hit_count >= *count)
            }
        }
    }
}
