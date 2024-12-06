use std::path::PathBuf;
use std::process::{Child, Command};
use windows_sys::Win32::System::Diagnostics::Debug::*;
use windows_sys::Win32::Foundation::*;

pub struct DebuggedProcess {
    process: Child,
    breakpoints: Vec<Breakpoint>,
    suspended: bool,
}

pub struct Breakpoint {
    address: usize,
    original_byte: u8,
    enabled: bool,
    line: u32,
    file: PathBuf,
}

impl DebuggedProcess {
    pub fn launch(program: &str, args: &[&str]) -> Result<Self, String> {
        let mut command = Command::new(program);
        command.args(args);
        command.creation_flags(DEBUG_PROCESS);

        let process = command.spawn().map_err(|e| e.to_string())?;

        Ok(DebuggedProcess {
            process,
            breakpoints: Vec::new(),
            suspended: false,
        })
    }

    pub fn set_breakpoint(&mut self, address: usize, line: u32, file: PathBuf) -> Result<(), String> {
        // Read original byte
        let mut original_byte = 0u8;
        unsafe {
            if ReadProcessMemory(
                self.process.as_raw_handle() as HANDLE,
                address as *const _,
                &mut original_byte as *mut _ as *mut _,
                1,
                std::ptr::null_mut(),
            ) == 0
            {
                return Err("Failed to read process memory".to_string());
            }
        }

        // Write int3 instruction (0xCC)
        let int3 = 0xCCu8;
        unsafe {
            if WriteProcessMemory(
                self.process.as_raw_handle() as HANDLE,
                address as *mut _,
                &int3 as *const _ as *const _,
                1,
                std::ptr::null_mut(),
            ) == 0
            {
                return Err("Failed to write breakpoint".to_string());
            }
        }

        self.breakpoints.push(Breakpoint {
            address,
            original_byte,
            enabled: true,
            line,
            file,
        });

        Ok(())
    }

    pub fn continue_execution(&mut self) -> Result<(), String> {
        if !self.suspended {
            return Ok(());
        }

        unsafe {
            if ContinueDebugEvent(
                self.process.id(),
                self.process.thread_id().unwrap_or(0),
                DBG_CONTINUE,
            ) == 0
            {
                return Err("Failed to continue process".to_string());
            }
        }

        self.suspended = false;
        Ok(())
    }

    pub fn step(&mut self) -> Result<(), String> {
        if !self.suspended {
            return Ok(());
        }

        // Set trap flag in EFLAGS register
        let mut context = CONTEXT {
            ContextFlags: CONTEXT_CONTROL,
            ..Default::default()
        };

        unsafe {
            if GetThreadContext(
                self.process.thread_id().unwrap_or(0) as HANDLE,
                &mut context,
            ) == 0
            {
                return Err("Failed to get thread context".to_string());
            }

            context.EFlags |= 0x100; // Set trap flag

            if SetThreadContext(
                self.process.thread_id().unwrap_or(0) as HANDLE,
                &context,
            ) == 0
            {
                return Err("Failed to set thread context".to_string());
            }
        }

        self.continue_execution()
    }

    pub fn wait_for_event(&mut self) -> Result<DebugEvent, String> {
        let mut debug_event = DEBUG_EVENT::default();

        unsafe {
            if WaitForDebugEvent(&mut debug_event, INFINITE) == 0 {
                return Err("Failed to wait for debug event".to_string());
            }
        }

        self.suspended = true;

        match debug_event.dwDebugEventCode {
            EXCEPTION_DEBUG_EVENT => {
                let exception = unsafe { debug_event.u.Exception };
                if exception.ExceptionRecord.ExceptionCode == EXCEPTION_BREAKPOINT {
                    // Handle breakpoint
                    if let Some(bp) = self.breakpoints.iter().find(|bp| {
                        bp.address == exception.ExceptionRecord.ExceptionAddress as usize
                    }) {
                        return Ok(DebugEvent::Breakpoint {
                            line: bp.line,
                            file: bp.file.clone(),
                        });
                    }
                }
                Ok(DebugEvent::Exception(exception.ExceptionRecord.ExceptionCode))
            }
            EXIT_PROCESS_DEBUG_EVENT => {
                let exit = unsafe { debug_event.u.ExitProcess };
                Ok(DebugEvent::ProcessExit(exit.dwExitCode))
            }
            _ => Ok(DebugEvent::Other(debug_event.dwDebugEventCode)),
        }
    }
}

#[derive(Debug)]
pub enum DebugEvent {
    Breakpoint {
        line: u32,
        file: PathBuf,
    },
    Exception(u32),
    ProcessExit(u32),
    Other(u32),
}
