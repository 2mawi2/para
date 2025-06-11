use anyhow::Result;
use para::config::manager::ConfigManager;
use para::core::session::manager::SessionManager;
use para::{Config, SessionStatus};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead, BufReader, Write};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    params: Option<Value>,
    id: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    result: Option<Value>,
    error: Option<JsonRpcError>,
    id: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    data: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct McpCapabilities {
    resources: bool,
    tools: bool,
    prompts: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct McpInitializeParams {
    protocol_version: String,
    capabilities: McpCapabilities,
    client_info: McpClientInfo,
}

#[derive(Debug, Serialize, Deserialize)]
struct McpClientInfo {
    name: String,
    version: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct McpTool {
    name: String,
    description: String,
    input_schema: Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct McpResource {
    uri: String,
    name: String,
    description: String,
    mime_type: Option<String>,
}

struct ParaMcpServer {
    config: Config,
    session_manager: SessionManager,
    initialized: bool,
}

impl ParaMcpServer {
    fn new() -> Result<Self> {
        let config = ConfigManager::load_or_create()
            .map_err(|e| anyhow::anyhow!("Failed to load config: {}", e))?;
        let session_manager = SessionManager::new(&config);

        Ok(Self {
            config: config.clone(),
            session_manager,
            initialized: false,
        })
    }

    fn handle_initialize(&mut self, _params: Value, id: Option<Value>) -> JsonRpcResponse {
        self.initialized = true;
        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(json!({
                "protocol_version": "2024-11-05",
                "capabilities": {
                    "resources": true,
                    "tools": true,
                    "prompts": false
                },
                "server_info": {
                    "name": "para-mcp-server",
                    "version": "1.1.2"
                }
            })),
            error: None,
            id,
        }
    }

    fn handle_tools_list(&self, id: Option<Value>) -> JsonRpcResponse {
        let tools = vec![
            McpTool {
                name: "para_start".to_string(),
                description: "Start a new isolated para session in separate Git worktree. Creates clean workspace for development with automatic branching.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Optional session name"
                        },
                        "prompt": {
                            "type": "string",
                            "description": "Optional initial prompt for the session"
                        }
                    }
                }),
            },
            McpTool {
                name: "para_finish".to_string(),
                description: "Complete current para session with commit message. REQUIRED to finish agent tasks. Auto-stages changes and prepares for integration.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "message": {
                            "type": "string",
                            "description": "Commit message for finishing the session"
                        }
                    },
                    "required": ["message"]
                }),
            },
            McpTool {
                name: "para_dispatch".to_string(),
                description: "ORCHESTRATION: Dispatch parallel AI agent to isolated worktree for independent task execution. Use for parallel development - each agent gets conflict-free workspace.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Unique session name for the dispatched agent (e.g., 'api-endpoints', 'frontend-ui')"
                        },
                        "prompt": {
                            "type": "string",
                            "description": "Complete task description for the agent. Include requirements, context, and remind agent to call 'para finish' when done."
                        },
                        "file": {
                            "type": "string",
                            "description": "Optional path to TASK_<number>_<description>.md file containing detailed task specification"
                        }
                    },
                    "required": ["name", "prompt"]
                }),
            },
            McpTool {
                name: "para_list".to_string(),
                description: "List all active para sessions and their status. Use to monitor parallel agent progress and coordinate team development.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            McpTool {
                name: "para_recover".to_string(),
                description: "Recover and resume work on a previous para session. Restores isolated worktree state for continuing development tasks.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Name of the session to recover"
                        }
                    },
                    "required": ["name"]
                }),
            },
            McpTool {
                name: "para_config_show".to_string(),
                description: "Display current para configuration including IDE settings, directory structure, and Git workflow preferences for development coordination.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
        ];

        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(json!({ "tools": tools })),
            error: None,
            id,
        }
    }

    fn handle_resources_list(&self, id: Option<Value>) -> Result<JsonRpcResponse> {
        let resources = vec![
            McpResource {
                uri: "para://current-session".to_string(),
                name: "Current Session".to_string(),
                description: "Information about the current para session".to_string(),
                mime_type: Some("application/json".to_string()),
            },
            McpResource {
                uri: "para://available-sessions".to_string(),
                name: "Available Sessions".to_string(),
                description: "List of all available para sessions".to_string(),
                mime_type: Some("application/json".to_string()),
            },
            McpResource {
                uri: "para://config".to_string(),
                name: "Para Configuration".to_string(),
                description: "Current para configuration settings".to_string(),
                mime_type: Some("application/json".to_string()),
            },
        ];

        Ok(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(json!({ "resources": resources })),
            error: None,
            id,
        })
    }

    fn handle_resources_read(&self, params: Value, id: Option<Value>) -> Result<JsonRpcResponse> {
        let uri = params["uri"].as_str().unwrap_or("");

        let result = match uri {
            "para://current-session" => {
                // Try to find current session based on current working directory
                let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                match self.session_manager.find_session_by_path(&current_dir) {
                    Ok(Some(session)) => json!({
                        "contents": [{
                            "uri": uri,
                            "mime_type": "application/json",
                            "text": serde_json::to_string_pretty(&session)?
                        }]
                    }),
                    Ok(None) => json!({
                        "contents": [{
                            "uri": uri,
                            "mime_type": "application/json",
                            "text": "{\"message\": \"No current session\"}"
                        }]
                    }),
                    Err(e) => json!({
                        "contents": [{
                            "uri": uri,
                            "mime_type": "application/json",
                            "text": format!("{{\"error\": \"{}\"}}", e)
                        }]
                    }),
                }
            }
            "para://available-sessions" => {
                let sessions = self
                    .session_manager
                    .list_sessions()
                    .map_err(|e| anyhow::anyhow!("Failed to list sessions: {}", e))?;
                json!({
                    "contents": [{
                        "uri": uri,
                        "mime_type": "application/json",
                        "text": serde_json::to_string_pretty(&sessions)?
                    }]
                })
            }
            "para://config" => {
                json!({
                    "contents": [{
                        "uri": uri,
                        "mime_type": "application/json",
                        "text": serde_json::to_string_pretty(&self.config)?
                    }]
                })
            }
            _ => {
                return Ok(JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32602,
                        message: format!("Unknown resource URI: {}", uri),
                        data: None,
                    }),
                    id,
                });
            }
        };

        Ok(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(result),
            error: None,
            id,
        })
    }

    fn handle_tools_call(&mut self, params: Value, id: Option<Value>) -> JsonRpcResponse {
        let tool_name = params["name"].as_str().unwrap_or("");
        let empty_map = serde_json::Map::new();
        let arguments = params["arguments"].as_object().unwrap_or(&empty_map);

        let result = match tool_name {
            "para_start" => self.handle_para_start(arguments),
            "para_finish" => self.handle_para_finish(arguments),
            "para_dispatch" => self.handle_para_dispatch(arguments),
            "para_list" => self.handle_para_list(),
            "para_recover" => self.handle_para_recover(arguments),
            "para_config_show" => self.handle_para_config_show(),
            _ => Err(anyhow::anyhow!("Unknown tool: {}", tool_name)),
        };

        match result {
            Ok(content) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: Some(json!({
                    "content": [{
                        "type": "text",
                        "text": content
                    }]
                })),
                error: None,
                id,
            },
            Err(e) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(JsonRpcError {
                    code: -32603,
                    message: e.to_string(),
                    data: None,
                }),
                id,
            },
        }
    }

    fn handle_para_start(&mut self, args: &serde_json::Map<String, Value>) -> Result<String> {
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                // Generate default session name
                let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
                format!("session-{}", timestamp)
            });
        let _prompt = args.get("prompt").and_then(|v| v.as_str());

        let session = self
            .session_manager
            .create_session(name, None)
            .map_err(|e| anyhow::anyhow!("Failed to create session: {}", e))?;
        Ok(format!(
            "Started session: {}\nPath: {}",
            session.name,
            session.worktree_path.display()
        ))
    }

    fn handle_para_finish(&mut self, args: &serde_json::Map<String, Value>) -> Result<String> {
        let message = args
            .get("message")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: message"))?;

        // Find current session and mark as finished
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        match self.session_manager.find_session_by_path(&current_dir)? {
            Some(session) => {
                self.session_manager
                    .update_session_status(&session.name, SessionStatus::Finished)
                    .map_err(|e| anyhow::anyhow!("Failed to finish session: {}", e))?;
                Ok(format!(
                    "Session '{}' finished with message: {}",
                    session.name, message
                ))
            }
            None => Err(anyhow::anyhow!(
                "No active session found in current directory"
            )),
        }
    }

    fn handle_para_dispatch(&mut self, args: &serde_json::Map<String, Value>) -> Result<String> {
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: name"))?;
        let _prompt = args
            .get("prompt")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: prompt"))?;
        let _file_path = args.get("file").and_then(|v| v.as_str()).map(PathBuf::from);

        // Create a new session for the dispatched agent
        let session = self
            .session_manager
            .create_session(name.to_string(), None)
            .map_err(|e| anyhow::anyhow!("Failed to dispatch session: {}", e))?;
        Ok(format!(
            "Dispatched agent session: {}\nPath: {}",
            session.name,
            session.worktree_path.display()
        ))
    }

    fn handle_para_list(&self) -> Result<String> {
        let sessions = self
            .session_manager
            .list_sessions()
            .map_err(|e| anyhow::anyhow!("Failed to list sessions: {}", e))?;
        if sessions.is_empty() {
            Ok("No sessions found".to_string())
        } else {
            let session_list: Vec<String> = sessions
                .iter()
                .map(|s| format!("- {} ({:?})", s.name, s.status))
                .collect();
            Ok(format!("Available sessions:\n{}", session_list.join("\n")))
        }
    }

    fn handle_para_recover(&mut self, args: &serde_json::Map<String, Value>) -> Result<String> {
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: name"))?;

        // Load the session state to "recover" it
        let session = self
            .session_manager
            .load_state(name)
            .map_err(|e| anyhow::anyhow!("Failed to recover session: {}", e))?;
        Ok(format!(
            "Recovered session: {}\nPath: {}",
            session.name,
            session.worktree_path.display()
        ))
    }

    fn handle_para_config_show(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(&self.config)?)
    }

    fn handle_request(&mut self, request: JsonRpcRequest) -> JsonRpcResponse {
        if !self.initialized && request.method != "initialize" {
            return JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(JsonRpcError {
                    code: -32002,
                    message: "Server not initialized".to_string(),
                    data: None,
                }),
                id: request.id,
            };
        }

        match request.method.as_str() {
            "initialize" => self.handle_initialize(request.params.unwrap_or(json!({})), request.id),
            "tools/list" => self.handle_tools_list(request.id),
            "tools/call" => self.handle_tools_call(request.params.unwrap_or(json!({})), request.id),
            "resources/list" => match self.handle_resources_list(request.id.clone()) {
                Ok(response) => response,
                Err(e) => JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32603,
                        message: e.to_string(),
                        data: None,
                    }),
                    id: request.id,
                },
            },
            "resources/read" => {
                match self
                    .handle_resources_read(request.params.unwrap_or(json!({})), request.id.clone())
                {
                    Ok(response) => response,
                    Err(e) => JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32603,
                            message: e.to_string(),
                            data: None,
                        }),
                        id: request.id,
                    },
                }
            }
            _ => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(JsonRpcError {
                    code: -32601,
                    message: format!("Method not found: {}", request.method),
                    data: None,
                }),
                id: request.id,
            },
        }
    }

    fn run(&mut self) -> Result<()> {
        let stdin = io::stdin();
        let reader = BufReader::new(stdin);
        let mut stdout = io::stdout();

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }

            let request: JsonRpcRequest = match serde_json::from_str(&line) {
                Ok(req) => req,
                Err(e) => {
                    let error_response = JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32700,
                            message: format!("Parse error: {}", e),
                            data: None,
                        }),
                        id: None,
                    };
                    writeln!(stdout, "{}", serde_json::to_string(&error_response)?)?;
                    continue;
                }
            };

            let response = self.handle_request(request);
            writeln!(stdout, "{}", serde_json::to_string(&response)?)?;
            stdout.flush()?;
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut server = ParaMcpServer::new()?;
    server.run()
}

#[cfg(test)]
mod tests {
    use super::*;
    use para::config::defaults::default_config;
    use serde_json::json;
    use tempfile::TempDir;

    fn create_test_server() -> Result<ParaMcpServer> {
        let temp_dir = TempDir::new().unwrap();
        
        let mut config = default_config();
        config.directories.state_dir = temp_dir.path().join(".para_state").to_string_lossy().to_string();
        config.directories.subtrees_dir = "test_subtrees".to_string();
        
        Ok(ParaMcpServer {
            config: config.clone(),
            session_manager: SessionManager::new(&config),
            initialized: false,
        })
    }

    #[test]
    fn test_mcp_server_initialize() {
        let mut server = create_test_server().unwrap();
        
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "initialize".to_string(),
            params: Some(json!({
                "protocol_version": "2024-11-05",
                "capabilities": {"resources": true, "tools": true},
                "client_info": {"name": "test", "version": "1.0"}
            })),
            id: Some(json!(1)),
        };

        let response = server.handle_request(request);
        
        assert_eq!(response.jsonrpc, "2.0");
        assert!(response.error.is_none());
        assert!(server.initialized);
        
        let result = response.result.unwrap();
        assert_eq!(result["protocol_version"], "2024-11-05");
        assert_eq!(result["server_info"]["name"], "para-mcp-server");
        assert_eq!(result["capabilities"]["tools"], true);
        assert_eq!(result["capabilities"]["resources"], true);
    }

    #[test]
    fn test_mcp_server_tools_list() {
        let mut server = create_test_server().unwrap();
        server.initialized = true;
        
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "tools/list".to_string(),
            params: None,
            id: Some(json!(2)),
        };

        let response = server.handle_request(request);
        
        assert_eq!(response.jsonrpc, "2.0");
        assert!(response.error.is_none());
        
        let result = response.result.unwrap();
        let tools = result["tools"].as_array().unwrap();
        
        assert_eq!(tools.len(), 6);
        
        let tool_names: Vec<String> = tools.iter()
            .map(|t| t["name"].as_str().unwrap().to_string())
            .collect();
        
        assert!(tool_names.contains(&"para_start".to_string()));
        assert!(tool_names.contains(&"para_finish".to_string()));
        assert!(tool_names.contains(&"para_dispatch".to_string()));
        assert!(tool_names.contains(&"para_list".to_string()));
        assert!(tool_names.contains(&"para_recover".to_string()));
        assert!(tool_names.contains(&"para_config_show".to_string()));
    }

    #[test]
    fn test_mcp_server_resources_list() {
        let mut server = create_test_server().unwrap();
        server.initialized = true;
        
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "resources/list".to_string(),
            params: None,
            id: Some(json!(3)),
        };

        let response = server.handle_request(request);
        
        assert_eq!(response.jsonrpc, "2.0");
        assert!(response.error.is_none());
        
        let result = response.result.unwrap();
        let resources = result["resources"].as_array().unwrap();
        
        assert_eq!(resources.len(), 3);
        
        let resource_uris: Vec<String> = resources.iter()
            .map(|r| r["uri"].as_str().unwrap().to_string())
            .collect();
        
        assert!(resource_uris.contains(&"para://current-session".to_string()));
        assert!(resource_uris.contains(&"para://available-sessions".to_string()));
        assert!(resource_uris.contains(&"para://config".to_string()));
    }

    #[test]
    fn test_mcp_server_config_resource() {
        let mut server = create_test_server().unwrap();
        server.initialized = true;
        
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "resources/read".to_string(),
            params: Some(json!({"uri": "para://config"})),
            id: Some(json!(4)),
        };

        let response = server.handle_request(request);
        
        assert_eq!(response.jsonrpc, "2.0");
        assert!(response.error.is_none());
        
        let result = response.result.unwrap();
        let contents = result["contents"].as_array().unwrap();
        assert_eq!(contents.len(), 1);
        
        let content = &contents[0];
        assert_eq!(content["uri"], "para://config");
        assert_eq!(content["mime_type"], "application/json");
        
        let config_text = content["text"].as_str().unwrap();
        let parsed_config: serde_json::Value = serde_json::from_str(config_text).unwrap();
        assert!(parsed_config["ide"].is_object());
        assert!(parsed_config["directories"].is_object());
    }

    #[test]
    fn test_mcp_server_para_list_tool() {
        let mut server = create_test_server().unwrap();
        server.initialized = true;
        
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "para_list",
                "arguments": {}
            })),
            id: Some(json!(5)),
        };

        let response = server.handle_request(request);
        
        assert_eq!(response.jsonrpc, "2.0");
        assert!(response.error.is_none());
        
        let result = response.result.unwrap();
        let content = &result["content"][0];
        assert_eq!(content["type"], "text");
        assert_eq!(content["text"], "No sessions found");
    }

    #[test]
    fn test_mcp_server_para_config_show_tool() {
        let mut server = create_test_server().unwrap();
        server.initialized = true;
        
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "para_config_show",
                "arguments": {}
            })),
            id: Some(json!(6)),
        };

        let response = server.handle_request(request);
        
        assert_eq!(response.jsonrpc, "2.0");
        assert!(response.error.is_none());
        
        let result = response.result.unwrap();
        let content = &result["content"][0];
        assert_eq!(content["type"], "text");
        
        let config_text = content["text"].as_str().unwrap();
        let parsed_config: serde_json::Value = serde_json::from_str(config_text).unwrap();
        assert!(parsed_config["ide"].is_object());
    }

    #[test]
    fn test_mcp_server_not_initialized_error() {
        let mut server = create_test_server().unwrap();
        // Don't initialize the server
        
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "tools/list".to_string(),
            params: None,
            id: Some(json!(7)),
        };

        let response = server.handle_request(request);
        
        assert_eq!(response.jsonrpc, "2.0");
        assert!(response.result.is_none());
        assert!(response.error.is_some());
        
        let error = response.error.unwrap();
        assert_eq!(error.code, -32002);
        assert_eq!(error.message, "Server not initialized");
    }

    #[test]
    fn test_mcp_server_unknown_method() {
        let mut server = create_test_server().unwrap();
        server.initialized = true;
        
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "unknown/method".to_string(),
            params: None,
            id: Some(json!(8)),
        };

        let response = server.handle_request(request);
        
        assert_eq!(response.jsonrpc, "2.0");
        assert!(response.result.is_none());
        assert!(response.error.is_some());
        
        let error = response.error.unwrap();
        assert_eq!(error.code, -32601);
        assert!(error.message.contains("Method not found"));
    }

    #[test]
    fn test_mcp_server_unknown_tool() {
        let mut server = create_test_server().unwrap();
        server.initialized = true;
        
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "unknown_tool",
                "arguments": {}
            })),
            id: Some(json!(9)),
        };

        let response = server.handle_request(request);
        
        assert_eq!(response.jsonrpc, "2.0");
        assert!(response.result.is_none());
        assert!(response.error.is_some());
        
        let error = response.error.unwrap();
        assert_eq!(error.code, -32603);
        assert!(error.message.contains("Unknown tool"));
    }

    #[test]
    fn test_mcp_server_unknown_resource() {
        let mut server = create_test_server().unwrap();
        server.initialized = true;
        
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "resources/read".to_string(),
            params: Some(json!({"uri": "para://unknown-resource"})),
            id: Some(json!(10)),
        };

        let response = server.handle_request(request);
        
        assert_eq!(response.jsonrpc, "2.0");
        assert!(response.result.is_none());
        assert!(response.error.is_some());
        
        let error = response.error.unwrap();
        assert_eq!(error.code, -32602);
        assert!(error.message.contains("Unknown resource URI"));
    }
}
