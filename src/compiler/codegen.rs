use llvm_sys::core::*;
use llvm_sys::prelude::*;
use llvm_sys::{LLVMContext, LLVMModule, LLVMBuilder};
use llvm_sys::debuginfo::*;
use std::collections::HashMap;
use std::ffi::CString;
use super::ast::*;

pub struct CodeGenerator {
    context: *mut LLVMContext,
    module: *mut LLVMModule,
    builder: *mut LLVMBuilder,
    di_builder: *mut LLVMDIBuilder,
    named_values: HashMap<String, LLVMValueRef>,
    debug_info: DebugInfo,
}

struct DebugInfo {
    compile_unit: LLVMMetadataRef,
    current_scope: LLVMMetadataRef,
    current_location: LLVMMetadataRef,
}

impl CodeGenerator {
    pub fn new(module_name: &str) -> Self {
        unsafe {
            let context = LLVMContextCreate();
            let module = LLVMModuleCreateWithNameInContext(
                CString::new(module_name).unwrap().as_ptr(),
                context
            );
            let builder = LLVMCreateBuilderInContext(context);

            // Initialize debug info
            let di_builder = LLVMCreateDIBuilder(module);
            let file = LLVMDIBuilderCreateFile(
                di_builder,
                module_name.as_ptr() as *const _,
                module_name.len(),
                "".as_ptr() as *const _,
                0,
            );
            
            let compile_unit = LLVMDIBuilderCreateCompileUnit(
                di_builder,
                DW_LANG_C,  // Using C as base language
                file,
                "Swift++ Compiler".as_ptr() as *const _,
                15,
                0,  // Not optimized
                "".as_ptr() as *const _,
                0,
                1,  // Debug version
                0,  // No flags
                0,  // Runtime version
                0,  // No split name
                0,  // No kind
                0,  // DWO id
                1,  // Split debug inlining
                0,  // Debug info for profiling
                0,  // No sys root
                0,  // No SDK
            );

            CodeGenerator {
                context,
                module,
                builder,
                di_builder,
                named_values: HashMap::new(),
                debug_info: DebugInfo {
                    compile_unit,
                    current_scope: file,
                    current_location: std::ptr::null_mut(),
                },
            }
        }
    }

    fn create_debug_location(&mut self, line: u32, column: u32) {
        unsafe {
            self.debug_info.current_location = LLVMDIBuilderCreateDebugLocation(
                self.context,
                line,
                column,
                self.debug_info.current_scope,
                std::ptr::null_mut(),
            );
            LLVMSetCurrentDebugLocation2(self.builder, self.debug_info.current_location);
        }
    }

    fn create_function_debug_info(&mut self, name: &str, line: u32) -> LLVMMetadataRef {
        unsafe {
            let function_type = LLVMDIBuilderCreateSubroutineType(
                self.di_builder,
                self.debug_info.compile_unit,
                std::ptr::null_mut(),
                0,
                0,
            );

            LLVMDIBuilderCreateFunction(
                self.di_builder,
                self.debug_info.current_scope,
                name.as_ptr() as *const _,
                name.len(),
                name.as_ptr() as *const _,
                name.len(),
                self.debug_info.compile_unit,
                line,
                function_type,
                0,  // Not local to unit
                1,  // Is definition
                line,
                LLVMDIFlagPrototyped as u32,
                0,  // Not optimized
            )
        }
    }

    fn create_variable_debug_info(&mut self, name: &str, ty: LLVMMetadataRef, line: u32, alloca: LLVMValueRef) {
        unsafe {
            let variable = LLVMDIBuilderCreateAutoVariable(
                self.di_builder,
                self.debug_info.current_scope,
                name.as_ptr() as *const _,
                name.len(),
                self.debug_info.compile_unit,
                line,
                ty,
                0,  // Not optimized
                0,  // No flags
                0,  // Alignment in bits
            );

            LLVMDIBuilderInsertDeclareAtEnd(
                self.di_builder,
                alloca,
                variable,
                LLVMDIBuilderCreateExpression(self.di_builder, std::ptr::null_mut(), 0),
                self.debug_info.current_location,
                LLVMGetInsertBlock(self.builder),
            );
        }
    }

    pub fn generate(&mut self, program: &Program) -> Result<(), String> {
        for statement in &program.statements {
            self.generate_statement(statement)?;
        }
        Ok(())
    }

    fn generate_statement(&mut self, statement: &Statement) -> Result<LLVMValueRef, String> {
        match statement {
            Statement::Function { name, params, return_type, body } => {
                self.generate_function(name, params, return_type, body)
            }
            Statement::Let { name, type_annotation: _, initializer } => {
                let value = self.generate_expression(initializer)?;
                self.named_values.insert(name.clone(), value);
                Ok(value)
            }
            Statement::Return(expr) => {
                if let Some(expr) = expr {
                    let value = self.generate_expression(expr)?;
                    unsafe {
                        Ok(LLVMBuildRet(self.builder, value))
                    }
                } else {
                    unsafe {
                        Ok(LLVMBuildRetVoid(self.builder))
                    }
                }
            }
            Statement::Expression(expr) => self.generate_expression(expr),
            _ => Err("Unsupported statement".to_string()),
        }
    }

    fn generate_function(
        &mut self,
        name: &str,
        params: &[(String, Type)],
        return_type: &Option<Type>,
        body: &[Statement]
    ) -> Result<LLVMValueRef, String> {
        unsafe {
            // Create function type
            let param_types: Vec<LLVMTypeRef> = params
                .iter()
                .map(|(_, ty)| self.type_to_llvm(ty))
                .collect();
            
            let return_type = if let Some(ty) = return_type {
                self.type_to_llvm(ty)
            } else {
                LLVMVoidTypeInContext(self.context)
            };

            let function_type = LLVMFunctionType(
                return_type,
                param_types.as_ptr(),
                param_types.len() as u32,
                0
            );

            // Create function
            let function = LLVMAddFunction(
                self.module,
                CString::new(name).unwrap().as_ptr(),
                function_type
            );

            // Create entry block
            let bb = LLVMAppendBasicBlockInContext(
                self.context,
                function,
                CString::new("entry").unwrap().as_ptr()
            );
            LLVMPositionBuilderAtEnd(self.builder, bb);

            // Add parameters to symbol table
            self.named_values.clear();
            for (i, (name, _)) in params.iter().enumerate() {
                let param = LLVMGetParam(function, i as u32);
                self.named_values.insert(name.clone(), param);
            }

            // Create function debug info
            let function_debug_info = self.create_function_debug_info(name, 1);

            // Generate function body
            for statement in body {
                self.generate_statement(statement)?;
            }

            // Verify function
            if LLVMVerifyFunction(function, LLVMVerifierFailureAction::LLVMPrintMessageAction) == 1 {
                return Err("Function verification failed".to_string());
            }

            Ok(function)
        }
    }

    fn generate_expression(&mut self, expr: &Expression) -> Result<LLVMValueRef, String> {
        match expr {
            Expression::Integer(value) => unsafe {
                Ok(LLVMConstInt(LLVMInt64TypeInContext(self.context), *value as u64, 0))
            },
            Expression::Float(value) => unsafe {
                Ok(LLVMConstReal(LLVMDoubleTypeInContext(self.context), *value))
            },
            Expression::Identifier(name) => {
                self.named_values
                    .get(name)
                    .copied()
                    .ok_or_else(|| format!("Unknown variable: {}", name))
            },
            Expression::Binary { left, operator, right } => {
                let l = self.generate_expression(left)?;
                let r = self.generate_expression(right)?;

                unsafe {
                    match operator {
                        BinaryOp::Add => Ok(LLVMBuildAdd(
                            self.builder,
                            l,
                            r,
                            CString::new("addtmp").unwrap().as_ptr()
                        )),
                        BinaryOp::Subtract => Ok(LLVMBuildSub(
                            self.builder,
                            l,
                            r,
                            CString::new("subtmp").unwrap().as_ptr()
                        )),
                        BinaryOp::Multiply => Ok(LLVMBuildMul(
                            self.builder,
                            l,
                            r,
                            CString::new("multmp").unwrap().as_ptr()
                        )),
                        BinaryOp::Divide => Ok(LLVMBuildSDiv(
                            self.builder,
                            l,
                            r,
                            CString::new("divtmp").unwrap().as_ptr()
                        )),
                        _ => Err("Unsupported binary operator".to_string()),
                    }
                }
            },
            Expression::Call { function, arguments } => {
                if let Expression::Identifier(name) = &**function {
                    unsafe {
                        let function = LLVMGetNamedFunction(
                            self.module,
                            CString::new(name.as_str()).unwrap().as_ptr()
                        );
                        
                        if function.is_null() {
                            return Err(format!("Unknown function: {}", name));
                        }

                        let mut args: Vec<LLVMValueRef> = Vec::new();
                        for arg in arguments {
                            args.push(self.generate_expression(arg)?);
                        }

                        Ok(LLVMBuildCall2(
                            self.builder,
                            LLVMTypeOf(function),
                            function,
                            args.as_mut_ptr(),
                            args.len() as u32,
                            CString::new("calltmp").unwrap().as_ptr()
                        ))
                    }
                } else {
                    Err("Invalid function call".to_string())
                }
            },
            _ => Err("Unsupported expression".to_string()),
        }
    }

    fn type_to_llvm(&self, ty: &Type) -> LLVMTypeRef {
        unsafe {
            match ty {
                Type::Int => LLVMInt64TypeInContext(self.context),
                Type::Float => LLVMDoubleTypeInContext(self.context),
                Type::Bool => LLVMInt1TypeInContext(self.context),
                Type::String => LLVMPointerType(LLVMInt8TypeInContext(self.context), 0),
                Type::Array(inner) => {
                    LLVMArrayType(self.type_to_llvm(inner), 0)
                },
                _ => LLVMVoidTypeInContext(self.context),
            }
        }
    }
}

impl Drop for CodeGenerator {
    fn drop(&mut self) {
        unsafe {
            LLVMDisposeDIBuilder(self.di_builder);
            LLVMDisposeBuilder(self.builder);
            LLVMDisposeModule(self.module);
            LLVMContextDispose(self.context);
        }
    }
}
