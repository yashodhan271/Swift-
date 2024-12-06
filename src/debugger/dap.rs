use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use async_trait::async_trait;
use dap::prelude::*;
use tokio::sync::mpsc;

use super::{Debugger, DebugEvent};

pub struct SwiftPPDebugAdapter {
    client: Client,
    debugger: Arc<Mutex<Debugger>>,
    event_receiver: mpsc::Receiver<DebugEvent>,
}

impl SwiftPPDebugAdapter {
    pub fn new(client: Client, event_receiver: mpsc::Receiver<DebugEvent>) -> Self {
        SwiftPPDebugAdapter {
            client,
            debugger: Arc::new(Mutex::new(Debugger::new())),
            event_receiver,
        }
    }

    async fn start_event_loop(&self) {
        let debugger = self.debugger.clone();
        let client = self.client.clone();
        let mut event_receiver = self.event_receiver.clone();

        tokio::spawn(async move {
            while let Some(event) = event_receiver.recv().await {
                let mut debugger = debugger.lock().unwrap();
                if let Err(e) = debugger.handle_debug_event(event).await {
                    let _ = client.send_event(Event::Output {
                        body: OutputEventBody {
                            category: Some("stderr".to_string()),
                            output: format!("Error handling debug event: {}", e),
                            ..Default::default()
                        },
                    });
                }
            }
        });
    }
}

#[async_trait]
impl DebuggerInterface for SwiftPPDebugAdapter {
    async fn initialize(&self, _args: InitializeRequestArguments) -> Result<Capabilities> {
        Ok(Capabilities {
            supports_configuration_done_request: Some(true),
            supports_function_breakpoints: Some(true),
            supports_conditional_breakpoints: Some(true),
            supports_hit_conditional_breakpoints: Some(true),
            supports_evaluate_for_hovers: Some(true),
            supports_step_back: Some(false),
            supports_set_variable: Some(true),
            supports_restart_frame: Some(false),
            supports_goto_targets_request: Some(false),
            supports_step_in_targets_request: Some(false),
            supports_completions_request: Some(true),
            completion_trigger_characters: Some(vec![".".to_string()]),
            supports_modules_request: Some(true),
            additional_module_columns: None,
            supported_checksum_algorithms: None,
            supports_restart_request: Some(true),
            supports_exception_options: Some(true),
            supports_value_formatting_options: Some(true),
            supports_exception_info_request: Some(true),
            support_terminate_debuggee: Some(true),
            support_suspend_debuggee: Some(true),
            supports_delayed_stack_trace_loading: Some(true),
            supports_loaded_sources_request: Some(true),
            supports_log_points: Some(true),
            supports_terminate_threads_request: Some(true),
            supports_set_expression: Some(true),
            supports_terminate_request: Some(true),
            supports_data_breakpoints: Some(true),
            supports_read_memory_request: Some(true),
            supports_write_memory_request: Some(true),
            supports_disassemble_request: Some(true),
            supports_cancel_request: Some(true),
            supports_breakpoint_locations_request: Some(true),
            supports_clipboard_context: Some(false),
            supports_stepping_granularity: Some(true),
            supports_instruction_breakpoints: Some(true),
            supports_exception_filter_options: Some(true),
            ..Capabilities::default()
        })
    }

    async fn launch(&self, args: LaunchRequestArguments) -> Result<()> {
        let program = args.get("program")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::new("Program path not specified"))?;

        let program_args = args.get("args")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter()
                .filter_map(|v| v.as_str())
                .map(String::from)
                .collect())
            .unwrap_or_default();

        let mut debugger = self.debugger.lock().unwrap();
        debugger.launch(PathBuf::from(program), program_args).await
            .map_err(|e| Error::new(e))?;

        self.start_event_loop().await;
        Ok(())
    }

    async fn set_breakpoints(&self, args: SetBreakpointsArguments) -> Result<SetBreakpointsResponse> {
        let source = args.source;
        let source_path = source.path
            .ok_or_else(|| Error::new("Source path not specified"))?;
        let breakpoints = args.breakpoints.unwrap_or_default();

        let mut verified_breakpoints = Vec::new();
        let mut debugger = self.debugger.lock().unwrap();

        for bp in breakpoints {
            let result = debugger.set_breakpoint(
                PathBuf::from(&source_path),
                bp.line as u32,
                bp.condition,
                bp.log_message,
            ).await;

            let verified = match result {
                Ok(id) => Breakpoint {
                    id: Some(id as i32),
                    verified: true,
                    message: None,
                    source: Some(source.clone()),
                    line: Some(bp.line),
                    column: bp.column,
                    end_line: None,
                    end_column: None,
                    instruction_reference: None,
                    offset: None,
                },
                Err(e) => Breakpoint {
                    verified: false,
                    message: Some(e),
                    source: Some(source.clone()),
                    line: Some(bp.line),
                    column: bp.column,
                    ..Breakpoint::default()
                },
            };

            verified_breakpoints.push(verified);
        }

        Ok(SetBreakpointsResponse {
            breakpoints: verified_breakpoints,
        })
    }

    async fn threads(&self) -> Result<ThreadsResponse> {
        // For now, just return the main thread
        Ok(ThreadsResponse {
            threads: vec![Thread {
                id: 1,
                name: "main".to_string(),
            }],
        })
    }

    async fn stack_trace(&self, _args: StackTraceArguments) -> Result<StackTraceResponse> {
        let debugger = self.debugger.lock().unwrap();
        let frames = debugger.get_backtrace().await
            .map_err(|e| Error::new(e))?;

        let stack_frames = frames.into_iter()
            .enumerate()
            .map(|(i, frame)| StackFrame {
                id: i as i32,
                name: frame.function_name,
                source: Some(Source {
                    name: Some(frame.source_file.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into()),
                    path: Some(frame.source_file.to_string_lossy().into()),
                    ..Source::default()
                }),
                line: frame.line as i32,
                column: frame.column as i32,
                ..StackFrame::default()
            })
            .collect();

        Ok(StackTraceResponse {
            stack_frames,
            total_frames: None,
        })
    }

    async fn continue_(&self) -> Result<ContinueResponse> {
        let mut debugger = self.debugger.lock().unwrap();
        debugger.continue_execution().await
            .map_err(|e| Error::new(e))?;

        Ok(ContinueResponse {
            all_threads_continued: Some(true),
        })
    }

    async fn next(&self, _args: NextArguments) -> Result<()> {
        let mut debugger = self.debugger.lock().unwrap();
        debugger.step_over().await
            .map_err(|e| Error::new(e))?;
        Ok(())
    }

    async fn step_in(&self, _args: StepInArguments) -> Result<()> {
        let mut debugger = self.debugger.lock().unwrap();
        debugger.step_into().await
            .map_err(|e| Error::new(e))?;
        Ok(())
    }

    async fn evaluate(&self, args: EvaluateArguments) -> Result<EvaluateResponse> {
        let debugger = self.debugger.lock().unwrap();
        let result = debugger.evaluate_expression(&args.expression).await
            .map_err(|e| Error::new(e))?;

        Ok(EvaluateResponse {
            result,
            type_: None,
            presentation_hint: None,
            variables_reference: 0,
            named_variables: None,
            indexed_variables: None,
            memory_reference: None,
        })
    }

    async fn disconnect(&self, _args: DisconnectArguments) -> Result<()> {
        // Cleanup will happen when the Debugger is dropped
        Ok(())
    }
}
