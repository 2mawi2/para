//! Docker authentication support for Claude Code
//!
//! This module handles platform-specific retrieval of Claude Code credentials
//! and provides them to Docker containers via environment variables.

use crate::core::docker::{DockerError, DockerResult};
use serde_json::Value;
use std::process::Command;

/// Trait for retrieving authentication credentials
pub trait AuthResolver {
    /// Get Claude credentials from the system
    fn get_claude_credentials(&self) -> DockerResult<ClaudeAuthTokens>;
}

/// Authentication tokens for Claude
#[derive(Debug)]
pub struct ClaudeAuthTokens {
    pub credentials_json: String,
}

/// macOS-specific auth resolver using Keychain
pub struct MacOSAuthResolver;

impl AuthResolver for MacOSAuthResolver {
    fn get_claude_credentials(&self) -> DockerResult<ClaudeAuthTokens> {
        // Get credentials from macOS keychain
        let output = Command::new("security")
            .args([
                "find-generic-password",
                "-s",
                "Claude Code-credentials",
                "-a",
                &std::env::var("USER").unwrap_or_else(|_| "".to_string()),
                "-w",
            ])
            .output()
            .map_err(|e| {
                DockerError::Other(anyhow::anyhow!("Failed to read from macOS Keychain: {}", e))
            })?;

        if !output.status.success() {
            return Err(DockerError::Other(anyhow::anyhow!(
                "Claude credentials not found in keychain. Please run 'claude /login' on your host machine first."
            )));
        }

        let creds_json = String::from_utf8(output.stdout).map_err(|e| {
            DockerError::Other(anyhow::anyhow!("Invalid UTF-8 in credentials: {}", e))
        })?;

        // Validate the JSON structure without deserializing into specific types
        // This ensures the credentials have all required fields that Claude expects
        let parsed: Value = serde_json::from_str(&creds_json).map_err(|e| {
            DockerError::Other(anyhow::anyhow!("Invalid JSON in credentials: {}", e))
        })?;

        // Ensure the top-level OAuth field exists
        let oauth = parsed.get("claudeAiOauth").ok_or_else(|| {
            DockerError::Other(anyhow::anyhow!(
                "Missing 'claudeAiOauth' field in credentials"
            ))
        })?;

        // Validate all required OAuth fields that Claude terminal app needs
        const REQUIRED_FIELDS: &[&str] = &[
            "accessToken",
            "refreshToken",
            "expiresAt",
            "scopes",
            "subscriptionType",
        ];

        for field in REQUIRED_FIELDS {
            if oauth.get(field).is_none() {
                return Err(DockerError::Other(anyhow::anyhow!(
                    "Missing required field '{}' in Claude OAuth credentials",
                    field
                )));
            }
        }

        Ok(ClaudeAuthTokens {
            credentials_json: creds_json,
        })
    }
}

/// Linux-specific auth resolver
pub struct LinuxAuthResolver;

impl AuthResolver for LinuxAuthResolver {
    fn get_claude_credentials(&self) -> DockerResult<ClaudeAuthTokens> {
        // For Linux, we'd need to implement a different strategy
        // For now, return an error indicating manual setup is needed
        Err(DockerError::Other(anyhow::anyhow!(
            "Automatic credential retrieval not yet implemented for Linux. Please set CLAUDE_ACCESS_TOKEN environment variable."
        )))
    }
}

/// Factory function to get the correct resolver for the current platform
pub fn get_auth_resolver() -> Box<dyn AuthResolver> {
    #[cfg(target_os = "macos")]
    {
        Box::new(MacOSAuthResolver)
    }
    #[cfg(not(target_os = "macos"))]
    {
        Box::new(LinuxAuthResolver)
    }
}
