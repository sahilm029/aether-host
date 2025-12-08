// src/bin/mock_tool.rs
use serde_json::Value;
use std::io::{self, BufRead, Write};

fn main() {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut handle = stdin.lock();

    let mut line = String::new();
    loop {
        line.clear();
        match handle.read_line(&mut line) {
            Ok(0) => break, // EOF
            Ok(_) => {
                if let Ok(json) = serde_json::from_str::<Value>(&line) {
                    let id = json["id"].as_u64().unwrap_or(1);
                    let method = json["method"].as_str().unwrap_or("");

                    // 1. INITIALIZE
                    if method == "initialize" {
                        let response = format!(
                            r#"{{"jsonrpc":"2.0","id":{},"result":{{"protocolVersion":"2024-11-05","capabilities":{{}},"serverInfo":{{"name":"MockTool","version":"1.0"}}}}}}"#,
                            id
                        );
                        send_response(&mut stdout, &response);
                    }
                    // 2. LIST TOOLS
                    else if method == "tools/list" {
                        let schema = serde_json::json!({
                            "type": "object",
                            "properties": {
                                "a": { "type": "number" },
                                "b": { "type": "number" }
                            },
                            "required": ["a", "b"]
                        });

                        let response = serde_json::json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "result": {
                                "tools": [{
                                    "name": "calculate_sum",
                                    "description": "Adds two numbers together",
                                    "inputSchema": schema
                                }]
                            }
                        });
                        send_response(&mut stdout, &response.to_string());
                    }
                    // 3. CALL TOOL (The Missing Piece!)
                    else if method == "tools/call" {
                        // Extract arguments
                        let params = &json["params"];
                        let tool_name = params["name"].as_str().unwrap_or("");
                        let args = &params["arguments"];

                        if tool_name == "calculate_sum" {
                            let a = args["a"].as_f64().unwrap_or(0.0);
                            let b = args["b"].as_f64().unwrap_or(0.0);
                            let sum = a + b;

                            // Return the result
                            let response = serde_json::json!({
                                "jsonrpc": "2.0",
                                "id": id,
                                "result": {
                                    "content": [{
                                        "type": "text",
                                        "text": format!("The sum is {}", sum)
                                    }]
                                }
                            });
                            send_response(&mut stdout, &response.to_string());
                        } else {
                            // Tool not found error
                            let response = serde_json::json!({
                                "jsonrpc": "2.0", "id": id,
                                "error": { "code": -32601, "message": "Method not found" }
                            });
                            send_response(&mut stdout, &response.to_string());
                        }
                    }
                }
            }
            Err(_) => break,
        }
    }
}

// Helper to write output + newline + flush
fn send_response(stdout: &mut std::io::Stdout, response: &str) {
    stdout.write_all(response.as_bytes()).unwrap();
    stdout.write_all(b"\n").unwrap(); // CRITICAL: The newline tells AETHER the message is done
    stdout.flush().unwrap();
}
