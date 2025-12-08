// src/client.rs
use crate::protocol::{
    ClientCapabilities, ClientInfo, InitializeParams, InitializeResult, JsonRpcRequest,
    JsonRpcResponse, ListToolsResult, Tool,
};
use crate::runtime::McpProcess;
use anyhow::{anyhow, Context, Result};
use crate::security::SecurityConfig;

pub struct McpClient {
    transport: McpProcess,
    request_id_counter: u64,
    security: SecurityConfig,
}

impl McpClient {
    // 1. Constructor: Wrap the process
    pub fn new(transport: McpProcess, config: SecurityConfig) -> Self {
        Self {
            transport,
            request_id_counter: 0,
            security: config,
        }
    }

    // 2. The Handshake Logic
    pub async fn initialize(&mut self) -> Result<()> {
        // A. Prepare the Payload
        let params = InitializeParams {
            protocol_version: "2024-11-05".to_string(),
            capabilities: ClientCapabilities { experimental: None },
            client_info: ClientInfo {
                name: "AETHER".to_string(),
                version: "0.1.0".to_string(),
            },
        };

        let request = JsonRpcRequest::new(
            "initialize",
            Some(serde_json::to_value(params)?),
            Some(self.next_id()),
        );

        // B. Send Request
        self.transport.send_request(&request).await?;

        // C. Wait for Response
        let response_str = self.transport.read_line().await?;

        // D. Parse Response
        let response: JsonRpcResponse = serde_json::from_str(&response_str)
            .context("Failed to parse init response from tool")?;

        // E. Check for Errors
        if let Some(err) = response.error {
            return Err(anyhow!(
                "MCP Init Error: {} (Code: {})",
                err.message,
                err.code
            ));
        }

        // F. Decode the Result
        if let Some(result) = response.result {
            let init_result: InitializeResult = serde_json::from_value(result)
                .context("Tool sent invalid initialize result format")?;

            println!("--- HANDSHAKE COMPLETE ---");
            println!(
                "Connected to: {} v{}",
                init_result.server_info.name, init_result.server_info.version
            );

            Ok(())
        } else {
            Err(anyhow!("Tool returned no result for initialize"))
        }
    }

    // Helper to generate IDs
    fn next_id(&mut self) -> u64 {
        self.request_id_counter += 1;
        self.request_id_counter
    }
    pub async fn list_tools(&mut self) -> Result<Vec<Tool>> {
        // 1. Send Request

        let request = JsonRpcRequest::new(
            "tools/list",
            None, // No params needed for listing
            Some(self.next_id()),
        );

        self.transport.send_request(&request).await?;

        // 2. Read Response

        let response_str = self.transport.read_line().await?;

        let response: JsonRpcResponse =
            serde_json::from_str(&response_str).context("Failed to parse tools/list response")?;

        // 3. Extract Result

        if let Some(result) = response.result {
            let tools_result: ListToolsResult =
                serde_json::from_value(result).context("Invalid tools list format")?;

            Ok(tools_result.tools)
        } else {
            Err(anyhow!("Server returned error or no result"))
        }
    }
    pub async fn call_tool(&mut self, tool_name: &str, arguments: serde_json::Value) -> Result<serde_json::Value> {
    // --- 1. THE SECURITY CHECK ---
        if !self.security.check_permission(tool_name) {
            return Err(anyhow::anyhow!("SECURITY ALERT: Tool '{}' is blocked by permissions.json", tool_name));
        }
        // -----------------------------

        // 2. Construct Request
        let params = serde_json::json!({
            "name": tool_name,
            "arguments": arguments
        });

        let request = JsonRpcRequest::new(
            "tools/call", // The standard MCP method to run a tool
            Some(params),
            Some(self.next_id()),
        );

        // 3. Send & Wait
        self.transport.send_request(&request).await?;
        let response_str = self.transport.read_line().await?;

        // 4. Parse Result
        let response: JsonRpcResponse = serde_json::from_str(&response_str)?;

        if let Some(err) = response.error {
            return Err(anyhow::anyhow!("Tool Execution Error: {}", err.message));
        }

        if let Some(result) = response.result {
            Ok(result)
        } else {
            Err(anyhow::anyhow!("Tool returned no result"))
        }
    }
}
