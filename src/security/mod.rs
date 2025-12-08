// src/security/mod.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use anyhow::{Result, Context};

#[derive(Debug, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub version: String,
    pub global_policy: String, // "allow" or "deny"
    pub rules: HashMap<String, String>, // Tool Name -> Action
}

impl SecurityConfig {
    // 1. Load from Disk
    pub fn load(path: &str) -> Result<Self> {
        let content = fs::read_to_string(path)
            .context(format!("Failed to read permissions file: {}", path))?;
        
        let config: SecurityConfig = serde_json::from_str(&content)
            .context("Failed to parse permissions.json")?;
            
        Ok(config)
    }

    // 2. The Check Logic (The Bouncer)
    pub fn check_permission(&self, tool_name: &str) -> bool {
        // Step A: Check specific rules first
        if let Some(policy) = self.rules.get(tool_name) {
            return policy == "allow";
        }

        // Step B: Fallback to global policy
        self.global_policy == "allow"
    }
}