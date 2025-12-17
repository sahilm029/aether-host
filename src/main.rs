// src/main.rs
use anyhow::Result;
use tokio::sync::mpsc;
use aether::llm::LlmClient;
use aether::runtime::McpProcess;
use aether::client::McpClient;
use aether::security::SecurityConfig;
use aether::tui::{self, App, UiMessage};
use aether::agent::Agent; // <--- Import your new Module

#[tokio::main]
async fn main() -> Result<()> {
    // 1. SETUP CHANNELS
    let (tx_agent, rx_agent) = mpsc::unbounded_channel::<String>();
    let (tx_ui, rx_ui) = mpsc::unbounded_channel::<UiMessage>();

    // 2. SETUP DEPENDENCIES
    // We do the dangerous setup here, but handle errors gracefully with '?'
    let security = SecurityConfig::load("permissions.json")?;
    
    // NOTE: Ensure this path matches your OS (Windows: mock_tool.exe)
    let tool_path = "target/debug/mock_tool.exe"; 
    let process = McpProcess::start(tool_path, &[])?;
    
    let mut client = McpClient::new(process, security);
    client.initialize().await?; // Handshake

    let llm = LlmClient::new("llama-3.3-70b-versatile")?;

    // 3. SPAWN THE BRAIN (Now just 2 lines!)
    tokio::spawn(async move {
        let agent = Agent::new(tx_ui, rx_agent, client, llm);
        agent.run().await;
    });

    // 4. START THE FACE
    let app = App::new(tx_agent);
    tui::run_tui(app, rx_ui).await?;

    Ok(())
}