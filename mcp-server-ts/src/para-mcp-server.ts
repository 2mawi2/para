#!/usr/bin/env node
/**
 * Para MCP Server - TypeScript implementation using official SDK
 * Calls into the Rust para binary for actual functionality
 */

import { Server } from "@modelcontextprotocol/sdk/server/index.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import {
  CallToolRequestSchema,
  ErrorCode,
  ListToolsRequestSchema,
  ListResourcesRequestSchema,
  ReadResourceRequestSchema,
  McpError,
} from "@modelcontextprotocol/sdk/types.js";
import { exec, execSync } from "child_process";
import { promisify } from "util";

const execAsync = promisify(exec);

// Para binary path - dynamically discover
function findParaBinary(): string {
  // Try multiple locations in order of preference
  const locations = [
    process.env.HOME + "/.local/bin/para",           // Local installation
    "/opt/homebrew/bin/para",                        // Apple Silicon Homebrew
    "/usr/local/bin/para",                           // Intel Mac Homebrew  
    "/home/linuxbrew/.linuxbrew/bin/para",          // Linux Homebrew
    "para"                                           // System PATH
  ];
  
  for (const location of locations) {
    try {
      execSync(`command -v ${location}`, { stdio: 'ignore' });
      return location;
    } catch {
      // Continue to next location
    }
  }
  
  // Fallback to 'para' in PATH
  return "para";
}

const PARA_BINARY = findParaBinary();

const server = new Server({
  name: "para-mcp-server",
  version: "1.1.2",
}, {
  capabilities: {
    tools: {},
    resources: {},
  }
});

// Helper function to execute para commands
async function runParaCommand(args: string[]): Promise<string> {
  try {
    const { stdout, stderr } = await execAsync(`${PARA_BINARY} ${args.join(' ')}`);
    if (stderr && !stderr.includes('warning')) {
      console.error(`Para command warning: ${stderr}`);
    }
    return stdout.trim();
  } catch (error: any) {
    throw new McpError(ErrorCode.InternalError, `Para command failed: ${error.message}`);
  }
}

// List available tools
server.setRequestHandler(ListToolsRequestSchema, async () => {
  return {
    tools: [
      {
        name: "para_start",
        description: "Start a new isolated para session in separate Git worktree. Creates clean workspace for development with automatic branching.",
        inputSchema: {
          type: "object",
          properties: {
            session_name: {
              type: "string",
              description: "Name for the new session"
            }
          },
          required: ["session_name"]
        }
      },
      {
        name: "para_finish",
        description: "Complete current para session and return to main branch. Auto-commits all changes with provided message.",
        inputSchema: {
          type: "object",
          properties: {
            commit_message: {
              type: "string",
              description: "Commit message for the changes"
            }
          },
          required: ["commit_message"]
        }
      },
      {
        name: "para_dispatch",
        description: "Start new para session with AI agent dispatch for parallel development.",
        inputSchema: {
          type: "object",
          properties: {
            session_name: {
              type: "string",
              description: "Name for the new session"
            },
            task_description: {
              type: "string",
              description: "Task description for the AI agent"
            }
          },
          required: ["session_name", "task_description"]
        }
      },
      {
        name: "para_list",
        description: "List all active para sessions with their status and branch information.",
        inputSchema: {
          type: "object",
          properties: {},
          additionalProperties: false
        }
      },
      {
        name: "para_recover",
        description: "Recover and resume a previous para session by name.",
        inputSchema: {
          type: "object",
          properties: {
            session_name: {
              type: "string",
              description: "Name of the session to recover"
            }
          },
          required: ["session_name"]
        }
      },
      {
        name: "para_config_show",
        description: "Display current para configuration including IDE, directories, and Git settings.",
        inputSchema: {
          type: "object",
          properties: {},
          additionalProperties: false
        }
      },
      {
        name: "para_cancel",
        description: "Cancel and delete a para session, removing its worktree and branch.",
        inputSchema: {
          type: "object",
          properties: {
            session_name: {
              type: "string",
              description: "Name of the session to cancel"
            }
          },
          required: ["session_name"]
        }
      }
    ]
  };
});

// Handle tool calls
server.setRequestHandler(CallToolRequestSchema, async (request) => {
  const { name, arguments: args } = request.params;

  try {
    let result: string;

    switch (name) {
      case "para_start":
        result = await runParaCommand(["start", (args as any).session_name]);
        break;
      
      case "para_finish":
        result = await runParaCommand(["finish", (args as any).commit_message]);
        break;
      
      case "para_dispatch":
        result = await runParaCommand(["dispatch", (args as any).session_name, (args as any).task_description]);
        break;
      
      case "para_list":
        result = await runParaCommand(["list"]);
        break;
      
      case "para_recover":
        result = await runParaCommand(["recover", (args as any).session_name]);
        break;
      
      case "para_config_show":
        result = await runParaCommand(["config", "show"]);
        break;
      
      case "para_cancel":
        result = await runParaCommand(["cancel", (args as any).session_name]);
        break;
      
      default:
        throw new McpError(ErrorCode.MethodNotFound, `Unknown tool: ${name}`);
    }

    return {
      content: [
        {
          type: "text",
          text: result
        }
      ]
    };
  } catch (error: any) {
    throw new McpError(ErrorCode.InternalError, `Tool execution failed: ${error.message}`);
  }
});

// List available resources
server.setRequestHandler(ListResourcesRequestSchema, async () => {
  return {
    resources: [
      {
        uri: "para://current-session",
        name: "Current Session",
        description: "Information about the current para session",
        mimeType: "application/json"
      },
      {
        uri: "para://config",
        name: "Para Configuration",
        description: "Current para configuration",
        mimeType: "application/json"
      }
    ]
  };
});

// Read resources
server.setRequestHandler(ReadResourceRequestSchema, async (request) => {
  const { uri } = request.params;

  try {
    let content: string;

    switch (uri) {
      case "para://current-session":
        content = await runParaCommand(["list", "--current"]);
        break;
      
      case "para://config":
        content = await runParaCommand(["config", "show"]);
        break;
      
      default:
        throw new McpError(ErrorCode.InvalidRequest, `Unknown resource: ${uri}`);
    }

    return {
      contents: [
        {
          uri,
          mimeType: "application/json",
          text: content
        }
      ]
    };
  } catch (error: any) {
    throw new McpError(ErrorCode.InternalError, `Resource read failed: ${error.message}`);
  }
});

// Start the server
async function main() {
  const transport = new StdioServerTransport();
  await server.connect(transport);
  console.error("Para MCP server running via TypeScript");
}

main().catch(console.error);