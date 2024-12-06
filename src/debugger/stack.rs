use std::path::PathBuf;
use windows_sys::Win32::System::Diagnostics::Debug::*;
use windows_sys::Win32::System::Threading::*;
use windows_sys::Win32::Foundation::*;

#[derive(Debug, Clone)]
pub struct StackFrame {
    pub instruction_pointer: usize,
    pub stack_pointer: usize,
    pub frame_pointer: usize,
    pub module_name: String,
    pub function_name: Option<String>,
    pub source_file: Option<PathBuf>,
    pub line_number: Option<u32>,
}

pub struct StackUnwinder {
    process_handle: HANDLE,
    thread_handle: HANDLE,
}

impl StackUnwinder {
    pub fn new(process_handle: HANDLE, thread_handle: HANDLE) -> Self {
        StackUnwinder {
            process_handle,
            thread_handle,
        }
    }

    pub fn unwind_stack(&self) -> Vec<StackFrame> {
        let mut frames = Vec::new();
        let mut context = CONTEXT {
            ContextFlags: CONTEXT_FULL,
            ..Default::default()
        };

        unsafe {
            // Get the current thread context
            if GetThreadContext(self.thread_handle, &mut context) == 0 {
                return frames;
            }

            let mut frame = StackFrame {
                instruction_pointer: context.Rip as usize,
                stack_pointer: context.Rsp as usize,
                frame_pointer: context.Rbp as usize,
                module_name: String::new(),
                function_name: None,
                source_file: None,
                line_number: None,
            };

            // Get module and symbol information
            self.populate_frame_info(&mut frame);
            frames.push(frame);

            // Set up stack frame for unwinding
            let mut stack_frame = STACKFRAME64 {
                AddrPC: ADDRESS64 {
                    Offset: context.Rip,
                    Segment: context.SegCs,
                    Mode: ADDRESS_MODE::AddrModeFlat as u32,
                },
                AddrFrame: ADDRESS64 {
                    Offset: context.Rbp,
                    Segment: context.SegSs,
                    Mode: ADDRESS_MODE::AddrModeFlat as u32,
                },
                AddrStack: ADDRESS64 {
                    Offset: context.Rsp,
                    Segment: context.SegSs,
                    Mode: ADDRESS_MODE::AddrModeFlat as u32,
                },
                ..Default::default()
            };

            // Unwind the stack
            while StackWalk64(
                IMAGE_FILE_MACHINE_AMD64,
                self.process_handle,
                self.thread_handle,
                &mut stack_frame,
                &mut context as *mut _ as *mut _,
                None,
                Some(SymFunctionTableAccess64),
                Some(SymGetModuleBase64),
                None,
            ) != 0
            {
                if stack_frame.AddrPC.Offset == 0 {
                    break;
                }

                let mut frame = StackFrame {
                    instruction_pointer: stack_frame.AddrPC.Offset as usize,
                    stack_pointer: stack_frame.AddrStack.Offset as usize,
                    frame_pointer: stack_frame.AddrFrame.Offset as usize,
                    module_name: String::new(),
                    function_name: None,
                    source_file: None,
                    line_number: None,
                };

                self.populate_frame_info(&mut frame);
                frames.push(frame);
            }
        }

        frames
    }

    fn populate_frame_info(&self, frame: &mut StackFrame) {
        unsafe {
            // Get module information
            let mut module_info = IMAGEHLP_MODULE64::default();
            module_info.SizeOfStruct = std::mem::size_of::<IMAGEHLP_MODULE64>() as u32;

            if SymGetModuleInfo64(
                self.process_handle,
                frame.instruction_pointer as u64,
                &mut module_info,
            ) != 0
            {
                frame.module_name = String::from_utf8_lossy(
                    &module_info.ModuleName[..module_info.ModuleName.iter().position(|&c| c == 0).unwrap_or(0)]
                ).to_string();
            }

            // Get symbol information
            let mut buffer = [0u8; std::mem::size_of::<SYMBOL_INFO>() + MAX_SYM_NAME * std::mem::size_of::<u8>()];
            let symbol = &mut *(buffer.as_mut_ptr() as *mut SYMBOL_INFO);
            symbol.SizeOfStruct = std::mem::size_of::<SYMBOL_INFO>() as u32;
            symbol.MaxNameLen = MAX_SYM_NAME as u32;

            let mut displacement = 0u64;

            if SymFromAddr(
                self.process_handle,
                frame.instruction_pointer as u64,
                &mut displacement,
                symbol,
            ) != 0
            {
                frame.function_name = Some(
                    String::from_utf8_lossy(&symbol.Name[..symbol.NameLen as usize]).to_string()
                );
            }

            // Get line information
            let mut line = IMAGEHLP_LINE64::default();
            line.SizeOfStruct = std::mem::size_of::<IMAGEHLP_LINE64>() as u32;
            let mut displacement = 0u32;

            if SymGetLineFromAddr64(
                self.process_handle,
                frame.instruction_pointer as u64,
                &mut displacement,
                &mut line,
            ) != 0
            {
                frame.source_file = Some(PathBuf::from(
                    String::from_utf8_lossy(
                        std::slice::from_raw_parts(
                            line.FileName as *const u8,
                            strlen(line.FileName),
                        )
                    ).to_string()
                ));
                frame.line_number = Some(line.LineNumber);
            }
        }
    }
}

unsafe fn strlen(ptr: *const i8) -> usize {
    let mut len = 0;
    while *ptr.add(len) != 0 {
        len += 1;
    }
    len
}
