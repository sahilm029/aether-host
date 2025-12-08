// src/llm.rs
use serde::{Deserialize, Serialize};
use serde_json::Value;
use anyhow::{Result, Context, anyhow};
use std::env;

// --- 1. THE GROQ API SHAPES ---

// The top-level request we send to Groq
#[derive(Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<GroqTool>>, 
}

// A single message in the conversation (User, Assistant, or Tool)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    // When WE send a tool result back, we need this field:
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>, 
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ToolCall {
    pub id: String,
    pub r#type: String, // "function"
    pub function: FunctionCall,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String, // Note: AI returns arguments as a STRING JSON
}

// --- 2. TOOL TRANSLATION LAYERS ---

// Groq expects tools wrapped in a specific way:
// { "type": "function", "function": { ... } }
#[derive(Serialize)]
struct GroqTool {
    r#type: String,
    function: GroqFunctionDefinition,
}

#[derive(Serialize)]
struct GroqFunctionDefinition {
    name: String,
    description: String,
    parameters: Value, // This is our input_schema
}

// --- 3. THE CLIENT ---

pub struct LlmClient {
    api_key: String,
    client: reqwest::Client,
    pub model: String,
}

impl LlmClient {
    pub fn new(model: &str) -> Result<Self> {
        // Load key from environment (Safety First!)
        dotenv::dotenv().ok();
        let api_key = env::var("GROQ_API_KEY")
            .context("GROQ_API_KEY not found in .env file")?;

        Ok(Self {
            api_key,
            client: reqwest::Client::new(),
            model: model.to_string(),
        })
    }

    // The Main Function: Send history -> Get Answer
    pub async fn send_completion(
        &self, 
        messages: &[Message], 
        tools: &[crate::protocol::Tool] // Take our internal tools
    ) -> Result<Message> {
        
        // A. Translate Tools (Our Struct -> Groq JSON)
        let groq_tools: Vec<GroqTool> = tools.iter().map(|t| {
            GroqTool {
                r#type: "function".to_string(),
                function: GroqFunctionDefinition {
                    name: t.name.clone(),
                    description: t.description.clone().unwrap_or_default(),
                    parameters: t.input_schema.clone(),
                }
            }
        }).collect();

        // B. Build Request
        let request = ChatCompletionRequest {
            model: self.model.clone(),
            messages: messages.to_vec(),
            tools: if groq_tools.is_empty() { None } else { Some(groq_tools) },
        };

        // C. Send HTTP Post
        let res = self.client.post("https://api.groq.com/openai/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Groq")?;

        // D. Parse Response
        if !res.status().is_success() {
            let error_text = res.text().await?;
            return Err(anyhow!("API Error: {}", error_text));
        }

        let response_json: Value = res.json().await?;
        
        // E. Extract the Message
        // Path: choices[0].message
        let message_value = response_json["choices"][0]["message"].clone();
        let message: Message = serde_json::from_value(message_value)
            .context("Failed to parse API response message")?;

        Ok(message)
    }
}