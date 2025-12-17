// src/agent.rs
use tokio::sync::mpsc;
use anyhow::{Result, anyhow};
use serde_json::Value;
use crate::{
    llm::{LlmClient, Message},
    client::McpClient,
    tui::UiMessage,
};

pub struct Agent {
    // The "Brain" needs to talk to the "Face" (UI)
    tx_ui: mpsc::UnboundedSender<UiMessage>,
    // The "Brain" needs to listen to the User
    rx_agent: mpsc::UnboundedReceiver<String>,
    // Dependencies
    client: McpClient,
    llm: LlmClient,
}

impl Agent {
    pub fn new(
        tx_ui: mpsc::UnboundedSender<UiMessage>,
        rx_agent: mpsc::UnboundedReceiver<String>,
        client: McpClient,
        llm: LlmClient,
    ) -> Self {
        Self { tx_ui, rx_agent, client, llm }
    }

    pub async fn run(mut self) {
        // Log startup
        self.log("Agent System Online.");
        
        // 1. Load Tools
        let tools = match self.client.list_tools().await {
            Ok(t) => {
                self.log(&format!("Tools Discovered: {}", t.len()));
                t
            },
            Err(e) => {
                self.error(&format!("Critical Tool Failure: {}", e));
                return; // Stop the agent safely
            }
        };

        // 2. Initialize History
        let mut history = vec![
            Message {
                role: "system".to_string(),
                content: Some("You are AETHER. Be concise. Use tools wisely.".to_string()),
                tool_calls: None,
                tool_call_id: None,
            }
        ];

        // 3. Main Loop (Waiting for user input)
        while let Some(user_input) = self.rx_agent.recv().await {
            self.log("Thinking...");
            
            // Add User Input
            history.push(Message {
                role: "user".to_string(),
                content: Some(user_input),
                tool_calls: None,
                tool_call_id: None,
            });

            // Run the ReAct Cycle
            if let Err(e) = self.cycle(&mut history, &tools).await {
                self.error(&format!("Cycle Error: {}", e));
            }
        }
    }

    // Isolate the logic for one "Turn" of conversation
    async fn cycle(&mut self, history: &mut Vec<Message>, tools: &[crate::protocol::Tool]) -> Result<()> {
        // A. Ask LLM
        let response = self.llm.send_completion(history, tools).await?;
        history.push(response.clone());

        // B. Check for Tools
        if let Some(tool_calls) = response.tool_calls {
            self.log(&format!("Tools Requested: {}", tool_calls.len()));

            for call in tool_calls {
                self.log(&format!("EXEC: {}({})", call.function.name, call.function.arguments));
                
                // Safe Argument Parsing (No unwrap)
                let args: Value = serde_json::from_str(&call.function.arguments)
                    .unwrap_or(serde_json::json!({})); 

                // Execute
                let result_str = match self.client.call_tool(&call.function.name, args).await {
                    Ok(res) => res.to_string(),
                    Err(e) => format!("Error: {}", e),
                };

                self.log(&format!("RESULT: {}", result_str));

                // Add Result to History
                history.push(Message {
                    role: "tool".to_string(),
                    content: Some(result_str),
                    tool_calls: None,
                    tool_call_id: Some(call.id),
                });
            }

            // C. Final Answer
            let final_res = self.llm.send_completion(history, &[]).await?;
            let text = final_res.content.clone().unwrap_or_else(|| "No content".to_string());
            
            self.send_ai(&text);
            history.push(final_res);
        } else {
            // No tools, just text
            let text = response.content.clone().unwrap_or_else(|| "No content".to_string());
            self.send_ai(&text);
        }

        Ok(())
    }

    // Helper to send Logs safely
    fn log(&self, msg: &str) {
        let _ = self.tx_ui.send(UiMessage::Log(msg.to_string()));
    }

    // Helper to send Errors safely
    fn error(&self, msg: &str) {
        let _ = self.tx_ui.send(UiMessage::Error(msg.to_string()));
    }

    // Helper to send AI replies
    fn send_ai(&self, msg: &str) {
        let _ = self.tx_ui.send(UiMessage::Ai(msg.to_string()));
    }
}