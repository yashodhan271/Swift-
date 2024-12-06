use std::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream as AsyncTcpStream;
use tokio::sync::mpsc;
use std::sync::{Arc, Mutex};

use super::protocol::*;

pub struct DebugClient {
    stream: AsyncTcpStream,
    sequence: Arc<Mutex<u64>>,
    event_sender: mpsc::Sender<DebugEvent>,
    event_receiver: mpsc::Receiver<DebugEvent>,
}

impl DebugClient {
    pub async fn connect(address: &str) -> Result<Self, String> {
        let stream = TcpStream::connect(address)
            .map_err(|e| format!("Failed to connect to debug server: {}", e))?;
        let stream = AsyncTcpStream::from_std(stream)
            .map_err(|e| format!("Failed to create async stream: {}", e))?;

        let (event_sender, event_receiver) = mpsc::channel(100);

        Ok(DebugClient {
            stream,
            sequence: Arc::new(Mutex::new(0)),
            event_sender,
            event_receiver,
        })
    }

    pub async fn send_command(&mut self, command: DebugCommand) -> Result<ResponseBody, String> {
        let seq = {
            let mut seq = self.sequence.lock().unwrap();
            *seq += 1;
            *seq
        };

        let message = ProtocolMessage {
            seq,
            typ: MessageType::Request,
            content: MessageContent::Command(command),
        };

        let json = serde_json::to_string(&message)
            .map_err(|e| format!("Failed to serialize command: {}", e))?;

        // Send message length followed by message
        let length = json.len() as u32;
        self.stream.write_all(&length.to_le_bytes()).await
            .map_err(|e| format!("Failed to write command length: {}", e))?;
        self.stream.write_all(json.as_bytes()).await
            .map_err(|e| format!("Failed to write command: {}", e))?;

        // Read response
        let mut length_buffer = [0u8; 4];
        self.stream.read_exact(&mut length_buffer).await
            .map_err(|e| format!("Failed to read response length: {}", e))?;
        let length = u32::from_le_bytes(length_buffer) as usize;

        let mut buffer = vec![0; length];
        self.stream.read_exact(&mut buffer).await
            .map_err(|e| format!("Failed to read response: {}", e))?;

        let response: ProtocolMessage = serde_json::from_slice(&buffer)
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        match response.content {
            MessageContent::Response { success, message, body, .. } => {
                if success {
                    body.ok_or_else(|| "No response body".to_string())
                } else {
                    Err(message.unwrap_or_else(|| "Unknown error".to_string()))
                }
            }
            _ => Err("Invalid response type".to_string()),
        }
    }

    pub async fn start_event_loop(&mut self) -> Result<(), String> {
        let mut buffer = Vec::new();
        let mut length_buffer = [0u8; 4];

        loop {
            // Read message length
            self.stream.read_exact(&mut length_buffer).await
                .map_err(|e| format!("Failed to read event length: {}", e))?;
            let length = u32::from_le_bytes(length_buffer) as usize;

            // Read message content
            buffer.resize(length, 0);
            self.stream.read_exact(&mut buffer).await
                .map_err(|e| format!("Failed to read event: {}", e))?;

            // Parse message
            let message: ProtocolMessage = serde_json::from_slice(&buffer)
                .map_err(|e| format!("Failed to parse event: {}", e))?;

            match message.content {
                MessageContent::Event(event) => {
                    self.event_sender.send(event).await
                        .map_err(|e| format!("Failed to send event: {}", e))?;
                }
                _ => {
                    return Err("Invalid message type received".to_string());
                }
            }
        }
    }

    pub async fn launch(&mut self, program: String, args: Vec<String>, working_dir: Option<String>) -> Result<(), String> {
        let response = self.send_command(DebugCommand::Launch {
            program,
            args,
            working_dir,
        }).await?;

        match response {
            ResponseBody::Capabilities(_) => Ok(()),
            _ => Err("Invalid response type".to_string()),
        }
    }

    pub async fn set_breakpoint(
        &mut self,
        file: std::path::PathBuf,
        line: u32,
        condition: Option<String>,
        log_message: Option<String>,
    ) -> Result<(), String> {
        let response = self.send_command(DebugCommand::SetBreakpoint {
            file,
            line,
            condition,
            log_message,
        }).await?;

        match response {
            ResponseBody::Capabilities(_) => Ok(()),
            _ => Err("Invalid response type".to_string()),
        }
    }

    pub async fn continue_execution(&mut self) -> Result<(), String> {
        self.send_command(DebugCommand::Continue).await?;
        Ok(())
    }

    pub async fn step_into(&mut self) -> Result<(), String> {
        self.send_command(DebugCommand::StepInto).await?;
        Ok(())
    }

    pub async fn step_over(&mut self) -> Result<(), String> {
        self.send_command(DebugCommand::StepOver).await?;
        Ok(())
    }

    pub async fn step_out(&mut self) -> Result<(), String> {
        self.send_command(DebugCommand::StepOut).await?;
        Ok(())
    }

    pub async fn get_stack_trace(&mut self) -> Result<Vec<StackFrame>, String> {
        let response = self.send_command(DebugCommand::GetStackTrace).await?;
        match response {
            ResponseBody::StackTrace(frames) => Ok(frames),
            _ => Err("Invalid response type".to_string()),
        }
    }

    pub async fn get_variables(&mut self, frame_id: usize) -> Result<Vec<Variable>, String> {
        let response = self.send_command(DebugCommand::GetVariables { frame_id }).await?;
        match response {
            ResponseBody::Variables(variables) => Ok(variables),
            _ => Err("Invalid response type".to_string()),
        }
    }

    pub async fn evaluate(&mut self, expression: String, frame_id: Option<usize>) -> Result<String, String> {
        let response = self.send_command(DebugCommand::Evaluate {
            expression,
            frame_id,
        }).await?;
        match response {
            ResponseBody::Evaluate(result) => Ok(result),
            _ => Err("Invalid response type".to_string()),
        }
    }
}
