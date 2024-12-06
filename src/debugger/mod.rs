pub mod breakpoint;
pub mod memory;
pub mod process;
pub mod remote;
pub mod stack;
pub mod symbols;
pub mod watchpoint;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

use crate::debugger::breakpoint::{Breakpoint, BreakpointManager, BreakpointType};
use crate::debugger::memory::MemoryInspector;
use crate::debugger::process::DebuggedProcess;
use crate::debugger::stack::StackUnwinder;
use crate::debugger::symbols::DebugSymbols;
use crate::debugger::watchpoint::{WatchpointManager, WatchType};

pub struct Debugger {
    process: Option<Arc<Mutex<DebuggedProcess>>>,
    breakpoint_manager: Arc<Mutex<BreakpointManager>>,
    watchpoint_manager: Arc<Mutex<WatchpointManager>>,
    memory_inspector: Arc<MemoryInspector>,
    stack_unwinder: Arc<StackUnwinder>,
    debug_symbols: Option<Arc<Mutex<DebugSymbols>>>,
    event_sender: mpsc::Sender<DebugEvent>,
    event_receiver: mpsc::Receiver<DebugEvent>,
}

#[derive(Debug)]
pub enum DebugEvent {
    ProcessStarted { pid: u32 },
    ProcessExited { exit_code: i32 },
    BreakpointHit { id: usize, thread_id: u32 },
    WatchpointHit { address: usize, old_value: Vec<u8>, new_value: Vec<u8> },
    Exception { code: u32, address: usize },
    ThreadCreated { thread_id: u32 },
    ThreadExited { thread_id: u32 },
    ModuleLoaded { name: String, base_address: usize },
    Output { message: String },
}

impl Debugger {
    pub fn new() -> Self {
        let (event_sender, event_receiver) = mpsc::channel(100);
        
        Debugger {
            process: None,
            breakpoint_manager: Arc::new(Mutex::new(BreakpointManager::new())),
            watchpoint_manager: Arc::new(Mutex::new(WatchpointManager::new(0))), // Handle will be set when process starts
            memory_inspector: Arc::new(MemoryInspector::new()),
            stack_unwinder: Arc::new(StackUnwinder::new()),
            debug_symbols: None,
            event_sender,
            event_receiver,
        }
    }

    pub async fn launch(&mut self, program: PathBuf, args: Vec<String>) -> Result<(), String> {
        // Create debugged process
        let process = DebuggedProcess::launch(&program, &args)
            .map_err(|e| format!("Failed to launch process: {}", e))?;
        
        // Load debug symbols
        let debug_symbols = DebugSymbols::new(&program)
            .map_err(|e| format!("Failed to load debug symbols: {}", e))?;
        
        let process = Arc::new(Mutex::new(process));
        
        // Initialize components with process handle
        {
            let proc = process.lock().unwrap();
            self.watchpoint_manager = Arc::new(Mutex::new(WatchpointManager::new(proc.handle())));
            self.memory_inspector = Arc::new(MemoryInspector::with_handle(proc.handle()));
            self.stack_unwinder = Arc::new(StackUnwinder::with_handle(proc.handle()));
        }
        
        self.process = Some(process);
        self.debug_symbols = Some(Arc::new(Mutex::new(debug_symbols)));
        
        self.event_sender.send(DebugEvent::ProcessStarted {
            pid: self.process.as_ref().unwrap().lock().unwrap().pid(),
        }).await.map_err(|e| format!("Failed to send event: {}", e))?;
        
        Ok(())
    }

    pub async fn set_breakpoint(
        &mut self,
        file: PathBuf,
        line: u32,
        condition: Option<String>,
        log_message: Option<String>,
    ) -> Result<usize, String> {
        let debug_symbols = self.debug_symbols.as_ref()
            .ok_or_else(|| "No debug symbols loaded".to_string())?;
        
        let symbols = debug_symbols.lock().unwrap();
        
        // Find address for the given file and line
        let address = symbols.find_address_for_location(&file, line)
            .ok_or_else(|| "Could not find address for breakpoint location".to_string())?;
        
        let breakpoint_type = if let Some(condition) = condition {
            BreakpointType::Conditional(condition)
        } else if let Some(message) = log_message {
            BreakpointType::LogPoint(message)
        } else {
            BreakpointType::Normal
        };
        
        let mut bp_manager = self.breakpoint_manager.lock().unwrap();
        bp_manager.add_breakpoint(address, line, file, breakpoint_type)
    }

    pub async fn set_watchpoint(
        &mut self,
        address: usize,
        size: usize,
        watch_type: WatchType,
    ) -> Result<(), String> {
        let mut wp_manager = self.watchpoint_manager.lock().unwrap();
        
        wp_manager.add_watchpoint(address, size, watch_type, move |addr, data| {
            println!("Watchpoint hit at 0x{:x}: {:?}", addr, data);
        })
    }

    pub async fn continue_execution(&mut self) -> Result<(), String> {
        let process = self.process.as_ref()
            .ok_or_else(|| "No active process".to_string())?;
        
        let mut process = process.lock().unwrap();
        process.continue_execution()
    }

    pub async fn step_into(&mut self) -> Result<(), String> {
        let process = self.process.as_ref()
            .ok_or_else(|| "No active process".to_string())?;
        
        let mut process = process.lock().unwrap();
        process.step_into()
    }

    pub async fn step_over(&mut self) -> Result<(), String> {
        let process = self.process.as_ref()
            .ok_or_else(|| "No active process".to_string())?;
        
        let mut process = process.lock().unwrap();
        process.step_over()
    }

    pub async fn get_backtrace(&self) -> Result<Vec<stack::StackFrame>, String> {
        let process = self.process.as_ref()
            .ok_or_else(|| "No active process".to_string())?;
        
        let process = process.lock().unwrap();
        self.stack_unwinder.unwind_stack(process.current_thread())
    }

    pub async fn read_memory(&self, address: usize, size: usize) -> Result<Vec<u8>, String> {
        self.memory_inspector.read_memory_range(address, size)
    }

    pub async fn write_memory(&self, address: usize, data: &[u8]) -> Result<(), String> {
        self.memory_inspector.write_memory_range(address, data)
    }

    pub async fn evaluate_expression(&self, expression: &str) -> Result<String, String> {
        // TODO: Implement expression evaluation using debug symbols and memory inspection
        Err("Expression evaluation not implemented".to_string())
    }

    pub async fn handle_debug_event(&mut self, event: DebugEvent) -> Result<(), String> {
        match event {
            DebugEvent::BreakpointHit { id, thread_id } => {
                let bp_manager = self.breakpoint_manager.lock().unwrap();
                if let Some(bp) = bp_manager.get_breakpoint(id) {
                    match &bp.breakpoint_type {
                        BreakpointType::Conditional(condition) => {
                            if self.evaluate_expression(condition).await.is_ok() {
                                println!("Breakpoint {} hit at {}:{}", id, bp.file.display(), bp.line);
                            } else {
                                self.continue_execution().await?;
                            }
                        }
                        BreakpointType::LogPoint(message) => {
                            println!("{}", message);
                            self.continue_execution().await?;
                        }
                        BreakpointType::Normal => {
                            println!("Breakpoint {} hit at {}:{}", id, bp.file.display(), bp.line);
                        }
                    }
                }
            }
            DebugEvent::WatchpointHit { address, old_value, new_value } => {
                println!("Watchpoint hit at 0x{:x}", address);
                println!("Old value: {:?}", old_value);
                println!("New value: {:?}", new_value);
            }
            _ => {
                // Handle other debug events
            }
        }
        Ok(())
    }
}
