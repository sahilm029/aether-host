// src/runtime/mod.rs
use tokio::process::{Command, Child, ChildStdin, ChildStdout};
use tokio::io::{AsyncWriteExt, AsyncBufReadExt, BufReader};
use std::process::Stdio;
use anyhow::{Result, Context, anyhow};
use crate::protocol::JsonRpcRequest; // Import our protocol

// The Structure that holds a running tool
pub struct McpProcess {
    // We keep the child handle so we can kill it later if needed
    pub child: Child, 
    // The "Pipe" we speak into
    pub stdin: ChildStdin, 
    // The "Ear" we listen to (Buffered for performance)
    pub stdout: BufReader<ChildStdout>, 
}

impl McpProcess {
    // 1. Spawn the Process
    pub fn start(command: &str, args: &[&str]) -> Result<Self> {
        let mut cmd = Command::new(command);
        cmd.args(args);

        // CRITICAL: We must "Pipe" the streams. 
        // If we don't do this, the child inherits OUR terminal.
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped()); // Capture errors too

        let mut child = cmd.spawn().context("Failed to spawn MCP tool")?;

        // 2. Extract the handles
        // We take() them because a child only has one stdin/stdout. 
        // Once we take them, they are ours.
        let stdin = child.stdin.take().ok_or(anyhow!("Failed to open stdin"))?;
        let stdout = child.stdout.take().ok_or(anyhow!("Failed to open stdout"))?;

        Ok(Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
        })
    }

    // 3. Send a Message
    // Note: We use &mut self because writing changes the state of the stream
    pub async fn send_request(&mut self, request: &JsonRpcRequest) -> Result<()> {
        // Serialize to JSON
        let mut json_string = serde_json::to_string(request)?;
        // MCP spec requires messages to be separated by newlines
        json_string.push('\n'); 

        // Write to the process's Stdin
        self.stdin.write_all(json_string.as_bytes()).await?;
        self.stdin.flush().await?; // Ensure it's actually sent
        
        Ok(())
    }

    // 4. Wait for ONE Response (Simple version)
    pub async fn read_line(&mut self) -> Result<String> {
        let mut line = String::new();
        // This waits until the process sends a "\n" character
        let bytes_read = self.stdout.read_line(&mut line).await?;
        
        if bytes_read == 0 {
            return Err(anyhow!("Process closed the connection (EOF)"));
        }
        
        Ok(line)
    }
}