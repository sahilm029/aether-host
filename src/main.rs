// src/main.rs
use anyhow::Result;
use tokio::sync::mpsc;
use aether::llm::{LlmClient, Message};
use aether::runtime::McpProcess;
use aether::client::McpClient;
use aether::security::SecurityConfig;
use aether::tui::{self, App, UiMessage};
use serde_json::Value;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. SETUP CHANNELS
    // UI -> Agent (User typed something)
    let (tx_agent, mut rx_agent) = mpsc::unbounded_channel::<String>();
    // Agent -> UI (AI replied or Log)
    let (tx_ui, rx_ui) = mpsc::unbounded_channel::<UiMessage>();

    // 2. SPAWN THE BRAIN (Background Task)
    // We clone tx_ui so the brain can talk to the face
    let tx_ui_clone = tx_ui.clone();
    
    tokio::spawn(async move {
        // --- BRAIN INITIALIZATION ---
        let _ = tx_ui_clone.send(UiMessage::Log("Initializing Core Systems...".into()));
        
        // A. Load Security
        let security = match SecurityConfig::load("permissions.json") {
            Ok(s) => s,
            Err(e) => {
                let _ = tx_ui_clone.send(UiMessage::Error(format!("Security Load Fail: {}", e)));
                return;
            }
        };

        // B. Connect Tool
        let tool_path = "target/debug/mock_tool.exe"; // Make sure this path is correct!
        let process = match McpProcess::start(tool_path, &[]) {
            Ok(p) => p,
            Err(e) => {
                let _ = tx_ui_clone.send(UiMessage::Error(format!("Tool Spawn Fail: {}", e)));
                return;
            }
        };

        // C. Handshake
        let mut client = McpClient::new(process, security);
        if let Err(e) = client.initialize().await {
            let _ = tx_ui_clone.send(UiMessage::Error(format!("Handshake Fail: {}", e)));
            return;
        }

        // D. List Tools
        let tools = match client.list_tools().await {
            Ok(t) => t,
            Err(e) => {
                let _ = tx_ui_clone.send(UiMessage::Error(format!("Tool List Fail: {}", e)));
                Vec::new()
            }
        };
        let _ = tx_ui_clone.send(UiMessage::Log(format!("Tools Loaded: {}", tools.len())));

        // E. Connect LLM
        let llm = match LlmClient::new("llama-3.3-70b-versatile") {
            Ok(l) => l,
            Err(e) => {
                let _ = tx_ui_clone.send(UiMessage::Error(format!("Groq Auth Fail: {}", e)));
                return;
            }
        };
        let _ = tx_ui_clone.send(UiMessage::Log("Neural Engine Online.".into()));

        // --- CONVERSATION LOOP ---
        // We keep a history for the AI
        let mut history = vec![
            Message {
                role: "system".to_string(),
                content: Some("You are AETHER. Use tools when needed. Keep answers concise.".to_string()),
                tool_calls: None,
                tool_call_id: None,
            }
        ];

        // Wait for user input from the UI
        while let Some(user_input) = rx_agent.recv().await {
            let _ = tx_ui_clone.send(UiMessage::Log("Processing Request...".into()));
            
            // 1. Add User Input to History
            history.push(Message {
                role: "user".to_string(),
                content: Some(user_input),
                tool_calls: None,
                tool_call_id: None,
            });

            // 2. Ask LLM
            let response = match llm.send_completion(&history, &tools).await {
                Ok(r) => r,
                Err(e) => {
                    let _ = tx_ui_clone.send(UiMessage::Error(format!("LLM Error: {}", e)));
                    continue;
                }
            };
            history.push(response.clone());

            // 3. Handle Tool Calls
            if let Some(tool_calls) = response.tool_calls {
                let _ = tx_ui_clone.send(UiMessage::Log(format!("AI Invoking {} Tools...", tool_calls.len())));

                for call in tool_calls {
                    let _ = tx_ui_clone.send(UiMessage::Log(format!("EXEC: {}({})", call.function.name, call.function.arguments)));
                    
                    // Parse Args
                    let args: Value = serde_json::from_str(&call.function.arguments).unwrap_or(serde_json::json!({}));

                    // Execute Tool
                    let result_str = match client.call_tool(&call.function.name, args).await {
                        Ok(res) => {
                            let s = res.to_string();
                            let _ = tx_ui_clone.send(UiMessage::Log(format!("RESULT: {}", s)));
                            s
                        },
                        Err(e) => {
                            let _ = tx_ui_clone.send(UiMessage::Error(format!("Tool Blocked/Failed: {}", e)));
                            format!("Error: {}", e)
                        }
                    };

                    // Feed back to history
                    history.push(Message {
                        role: "tool".to_string(),
                        content: Some(result_str),
                        tool_calls: None,
                        tool_call_id: Some(call.id),
                    });
                }

                // Final Answer after tools
                if let Ok(final_res) = llm.send_completion(&history, &[]).await {
                    let text = final_res.content.clone().unwrap_or_default();
                    let _ = tx_ui_clone.send(UiMessage::Ai(text.clone()));
                    history.push(final_res);
                }
            } else {
                // No tools, just text
                let text = response.content.clone().unwrap_or_default();
                let _ = tx_ui_clone.send(UiMessage::Ai(text));
            }
        }
    });

    // 3. START THE FACE (Main Thread)
    // We pass the "tx_agent" so the UI can send messages to the brain
    let app = App::new(tx_agent);
    tui::run_tui(app, rx_ui).await?;

    Ok(())
}