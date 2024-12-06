use std::collections::HashMap;
use super::ast::*;

#[derive(Debug, Clone)]
pub struct TypeEnvironment {
    variables: HashMap<String, Type>,
    functions: HashMap<String, FunctionType>,
    structs: HashMap<String, StructType>,
}

#[derive(Debug, Clone)]
pub struct FunctionType {
    params: Vec<(String, Type)>,
    return_type: Option<Type>,
}

#[derive(Debug, Clone)]
pub struct StructType {
    fields: HashMap<String, Type>,
}

pub struct SemanticAnalyzer {
    environment: TypeEnvironment,
    errors: Vec<String>,
}

impl SemanticAnalyzer {
    pub fn new() -> Self {
        SemanticAnalyzer {
            environment: TypeEnvironment {
                variables: HashMap::new(),
                functions: HashMap::new(),
                structs: HashMap::new(),
            },
            errors: Vec::new(),
        }
    }

    pub fn analyze(&mut self, program: &Program) -> Result<(), Vec<String>> {
        // First pass: collect all type declarations
        self.collect_declarations(program);
        
        // Second pass: analyze statements and expressions
        for statement in &program.statements {
            self.analyze_statement(statement)?;
        }
        
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.clone())
        }
    }

    fn collect_declarations(&mut self, program: &Program) {
        for statement in &program.statements {
            match statement {
                Statement::Function { name, params, return_type, .. } => {
                    self.environment.functions.insert(
                        name.clone(),
                        FunctionType {
                            params: params.clone(),
                            return_type: return_type.clone(),
                        },
                    );
                }
                Statement::Struct { name, fields } => {
                    let mut field_types = HashMap::new();
                    for (field_name, field_type) in fields {
                        field_types.insert(field_name.clone(), field_type.clone());
                    }
                    self.environment.structs.insert(
                        name.clone(),
                        StructType { fields: field_types },
                    );
                }
                _ => {}
            }
        }
    }

    fn analyze_statement(&mut self, statement: &Statement) -> Result<(), Vec<String>> {
        match statement {
            Statement::Let { name, type_annotation, initializer } => {
                let expr_type = self.analyze_expression(initializer)?;
                
                if let Some(annotated_type) = type_annotation {
                    if !self.types_match(&expr_type, annotated_type) {
                        self.errors.push(format!(
                            "Type mismatch: expected {:?}, found {:?}",
                            annotated_type, expr_type
                        ));
                    }
                }
                
                self.environment.variables.insert(name.clone(), expr_type);
            }
            
            Statement::Function { name, params, return_type, body } => {
                // Create new scope for function body
                let mut function_env = self.environment.clone();
                
                // Add parameters to function scope
                for (param_name, param_type) in params {
                    function_env.variables.insert(param_name.clone(), param_type.clone());
                }
                
                // Analyze function body
                let mut analyzer = SemanticAnalyzer {
                    environment: function_env,
                    errors: Vec::new(),
                };
                
                for stmt in body {
                    analyzer.analyze_statement(stmt)?;
                }
                
                // Merge errors
                self.errors.extend(analyzer.errors);
            }
            
            Statement::Return(expr) => {
                if let Some(expr) = expr {
                    self.analyze_expression(expr)?;
                }
            }
            
            Statement::Expression(expr) => {
                self.analyze_expression(expr)?;
            }
            
            Statement::If { condition, then_branch, else_branch } => {
                let condition_type = self.analyze_expression(condition)?;
                if !matches!(condition_type, Type::Bool) {
                    self.errors.push("If condition must be a boolean".to_string());
                }
                
                for stmt in then_branch {
                    self.analyze_statement(stmt)?;
                }
                
                if let Some(else_branch) = else_branch {
                    for stmt in else_branch {
                        self.analyze_statement(stmt)?;
                    }
                }
            }
            
            Statement::While { condition, body } => {
                let condition_type = self.analyze_expression(condition)?;
                if !matches!(condition_type, Type::Bool) {
                    self.errors.push("While condition must be a boolean".to_string());
                }
                
                for stmt in body {
                    self.analyze_statement(stmt)?;
                }
            }
            
            _ => {}
        }
        
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.clone())
        }
    }

    fn analyze_expression(&mut self, expr: &Expression) -> Result<Type, Vec<String>> {
        match expr {
            Expression::Integer(_) => Ok(Type::Int),
            Expression::Float(_) => Ok(Type::Float),
            Expression::String(_) => Ok(Type::String),
            Expression::Boolean(_) => Ok(Type::Bool),
            
            Expression::Identifier(name) => {
                self.environment.variables
                    .get(name)
                    .cloned()
                    .ok_or_else(|| vec![format!("Undefined variable: {}", name)])
            }
            
            Expression::Binary { left, operator, right } => {
                let left_type = self.analyze_expression(left)?;
                let right_type = self.analyze_expression(right)?;
                
                if !self.types_match(&left_type, &right_type) {
                    self.errors.push(format!(
                        "Binary operation type mismatch: {:?} {} {:?}",
                        left_type, operator, right_type
                    ));
                }
                
                match operator {
                    BinaryOp::Add | BinaryOp::Subtract | BinaryOp::Multiply | BinaryOp::Divide => {
                        Ok(left_type)
                    }
                    BinaryOp::Equal | BinaryOp::NotEqual | BinaryOp::Greater | BinaryOp::Less |
                    BinaryOp::GreaterEqual | BinaryOp::LessEqual => {
                        Ok(Type::Bool)
                    }
                }
            }
            
            Expression::Call { function, arguments } => {
                if let Expression::Identifier(name) = &**function {
                    if let Some(func_type) = self.environment.functions.get(name) {
                        if arguments.len() != func_type.params.len() {
                            self.errors.push(format!(
                                "Wrong number of arguments: expected {}, found {}",
                                func_type.params.len(), arguments.len()
                            ));
                        }
                        
                        for (arg, (_, param_type)) in arguments.iter().zip(&func_type.params) {
                            let arg_type = self.analyze_expression(arg)?;
                            if !self.types_match(&arg_type, param_type) {
                                self.errors.push(format!(
                                    "Argument type mismatch: expected {:?}, found {:?}",
                                    param_type, arg_type
                                ));
                            }
                        }
                        
                        Ok(func_type.return_type.clone().unwrap_or(Type::Int))
                    } else {
                        Err(vec![format!("Undefined function: {}", name)])
                    }
                } else {
                    Err(vec!["Invalid function call".to_string()])
                }
            }
            
            _ => Err(vec!["Unsupported expression".to_string()]),
        }
    }

    fn types_match(&self, t1: &Type, t2: &Type) -> bool {
        match (t1, t2) {
            (Type::Int, Type::Int) |
            (Type::Float, Type::Float) |
            (Type::String, Type::String) |
            (Type::Bool, Type::Bool) => true,
            
            (Type::Custom(name1), Type::Custom(name2)) => name1 == name2,
            
            (Type::Array(inner1), Type::Array(inner2)) => 
                self.types_match(inner1, inner2),
            
            (Type::Function(params1, ret1), Type::Function(params2, ret2)) => {
                params1.len() == params2.len() &&
                params1.iter().zip(params2).all(|(p1, p2)| self.types_match(p1, p2)) &&
                self.types_match(ret1, ret2)
            }
            
            _ => false,
        }
    }
}
