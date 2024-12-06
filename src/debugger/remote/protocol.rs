use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub enum DebugCommand {
    Launch {
        program: String,
        args: Vec<String>,
        working_dir: Option<String>,
    },
    Attach {
        pid: u32,
    },
    Continue,
    StepInto,
    StepOver,
    StepOut,
    Pause,
    SetBreakpoint {
        file: PathBuf,
        line: u32,
        condition: Option<String>,
        log_message: Option<String>,
    },
    RemoveBreakpoint {
        id: usize,
    },
    ReadMemory {
        address: usize,
        size: usize,
    },
    WriteMemory {
        address: usize,
        data: Vec<u8>,
    },
    GetStackTrace,
    GetVariables {
        frame_id: usize,
    },
    Evaluate {
        expression: String,
        frame_id: Option<usize>,
    },
    SetVariable {
        frame_id: usize,
        name: String,
        value: String,
    },
    Disconnect,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum DebugEvent {
    Started {
        pid: u32,
    },
    Stopped {
        reason: StopReason,
        thread_id: u32,
        frame_id: Option<usize>,
    },
    Breakpoint {
        id: usize,
        verified: bool,
        message: Option<String>,
    },
    Output {
        category: OutputCategory,
        message: String,
    },
    ThreadCreated {
        thread_id: u32,
    },
    ThreadExited {
        thread_id: u32,
    },
    ModuleLoaded {
        name: String,
        path: PathBuf,
    },
    Error {
        message: String,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum StopReason {
    Breakpoint { id: usize },
    Step,
    Pause,
    Exception { description: String },
    Entry,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum OutputCategory {
    Console,
    Stdout,
    Stderr,
    Debug,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StackFrame {
    pub id: usize,
    pub name: String,
    pub source: Option<Source>,
    pub line: u32,
    pub column: u32,
    pub module: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Source {
    pub path: PathBuf,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Variable {
    pub name: String,
    pub value: String,
    pub type_name: String,
    pub variables_reference: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DebuggerCapabilities {
    pub supports_step_back: bool,
    pub supports_restart_frame: bool,
    pub supports_goto_targets: bool,
    pub supports_evaluate_for_hovers: bool,
    pub supports_conditional_breakpoints: bool,
    pub supports_log_points: bool,
    pub supports_set_variable: bool,
    pub supports_completions: bool,
    pub supports_modules: bool,
    pub supports_memory_references: bool,
    pub supports_value_formatting: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProtocolMessage {
    pub seq: u64,
    pub typ: MessageType,
    pub content: MessageContent,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum MessageType {
    Request,
    Response,
    Event,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum MessageContent {
    Command(DebugCommand),
    Event(DebugEvent),
    Response {
        request_seq: u64,
        success: bool,
        message: Option<String>,
        body: Option<ResponseBody>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ResponseBody {
    StackTrace(Vec<StackFrame>),
    Variables(Vec<Variable>),
    Memory(Vec<u8>),
    Evaluate(String),
    Capabilities(DebuggerCapabilities),
}
