use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream as AsyncTcpStream;
use tokio::sync::mpsc;

use super::protocol::*;
use crate::debugger::{process::DebuggedProcess, memory::MemoryInspector, stack::StackUnwinder};

pub struct DebugServer {
    listener: TcpListener,
    sequence: Arc<Mutex<u64>>,
    debugged_process: Option<Arc<Mutex<DebuggedProcess>>>,
    event_sender: mpsc::Sender<DebugEvent>,
    event_receiver: mpsc::Receiver<DebugEvent>,
}

impl DebugServer {
    pub fn new(address: &str) -> Result<Self, String> {
        let listener = TcpListener::bind(address)
            .map_err(|e| format!("Failed to bind to address: {}", e))?;

        let (event_sender, event_receiver) = mpsc::channel(100);

        Ok(DebugServer {
            listener,
            sequence: Arc::new(Mutex::new(0)),
            debugged_process: None,
            event_sender,
            event_receiver,
        })
    }

    pub async fn run(&mut self) -> Result<(), String> {
        println!("Debug server listening...");

        for stream in self.listener.incoming() {
            match stream {
                Ok(stream) => {
                    let sequence = self.sequence.clone();
                    let event_sender = self.event_sender.clone();
                    let debugged_process = self.debugged_process.clone();

                    tokio::spawn(async move {
                        if let Err(e) = handle_client(stream, sequence, event_sender, debugged_process).await {
                            eprintln!("Error handling client: {}", e);
                        }
                    });
                }
                Err(e) => {
                    eprintln!("Error accepting connection: {}", e);
                }
            }
        }

        Ok(())
    }

    async fn handle_event(&mut self, event: DebugEvent) -> Result<(), String> {
        let seq = {
            let mut seq = self.sequence.lock().unwrap();
            *seq += 1;
            *seq
        };

        let message = ProtocolMessage {
            seq,
            typ: MessageType::Event,
            content: MessageContent::Event(event),
        };

        // Broadcast event to all connected clients
        // In a real implementation, you'd maintain a list of connected clients
        let json = serde_json::to_string(&message)
            .map_err(|e| format!("Failed to serialize event: {}", e))?;

        // Send event to connected clients
        Ok(())
    }
}

async fn handle_client(
    stream: TcpStream,
    sequence: Arc<Mutex<u64>>,
    event_sender: mpsc::Sender<DebugEvent>,
    debugged_process: Option<Arc<Mutex<DebuggedProcess>>>,
) -> Result<(), String> {
    let mut stream = AsyncTcpStream::from_std(stream)
        .map_err(|e| format!("Failed to create async stream: {}", e))?;

    let mut buffer = Vec::new();
    let mut length_buffer = [0u8; 4];

    loop {
        // Read message length
        stream.read_exact(&mut length_buffer).await
            .map_err(|e| format!("Failed to read message length: {}", e))?;
        let length = u32::from_le_bytes(length_buffer) as usize;

        // Read message content
        buffer.resize(length, 0);
        stream.read_exact(&mut buffer).await
            .map_err(|e| format!("Failed to read message: {}", e))?;

        // Parse message
        let message: ProtocolMessage = serde_json::from_slice(&buffer)
            .map_err(|e| format!("Failed to parse message: {}", e))?;

        // Handle message
        match message.content {
            MessageContent::Command(cmd) => {
                let response = handle_command(cmd, &debugged_process, &event_sender).await?;
                
                // Send response
                let response_message = ProtocolMessage {
                    seq: {
                        let mut seq = sequence.lock().unwrap();
                        *seq += 1;
                        *seq
                    },
                    typ: MessageType::Response,
                    content: MessageContent::Response {
                        request_seq: message.seq,
                        success: true,
                        message: None,
                        body: Some(response),
                    },
                };

                let json = serde_json::to_string(&response_message)
                    .map_err(|e| format!("Failed to serialize response: {}", e))?;
                
                let length = json.len() as u32;
                stream.write_all(&length.to_le_bytes()).await
                    .map_err(|e| format!("Failed to write response length: {}", e))?;
                stream.write_all(json.as_bytes()).await
                    .map_err(|e| format!("Failed to write response: {}", e))?;
            }
            _ => {
                return Err("Invalid message type received".to_string());
            }
        }
    }
}

async fn handle_command(
    command: DebugCommand,
    debugged_process: &Option<Arc<Mutex<DebuggedProcess>>>,
    event_sender: &mpsc::Sender<DebugEvent>,
) -> Result<ResponseBody, String> {
    match command {
        DebugCommand::Launch { program, args, working_dir } => {
            // Launch process implementation
            Ok(ResponseBody::Capabilities(DebuggerCapabilities {
                supports_step_back: false,
                supports_restart_frame: false,
                supports_goto_targets: false,
                supports_evaluate_for_hovers: true,
                supports_conditional_breakpoints: true,
                supports_log_points: true,
                supports_set_variable: true,
                supports_completions: true,
                supports_modules: true,
                supports_memory_references: true,
                supports_value_formatting: true,
            }))
        }
        DebugCommand::GetStackTrace => {
            if let Some(process) = debugged_process {
                let process = process.lock().unwrap();
                // Get stack trace implementation
                Ok(ResponseBody::StackTrace(vec![]))
            } else {
                Err("No active debug session".to_string())
            }
        }
        // Implement other commands
        _ => Err("Command not implemented".to_string()),
    }
}
