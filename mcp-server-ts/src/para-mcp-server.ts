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

// Para binary path - dynamically discover
function findParaBinary(): string {
  // Check if MCP server is running from homebrew
  const mcpPath = process.argv[1]; // Path to this script
  const isHomebrewMcp = mcpPath && (mcpPath.includes('/homebrew/') || mcpPath.includes('/usr/local/'));
  
  if (isHomebrewMcp) {
    // For homebrew MCP server, only use homebrew para
    const homebrewLocations = [
      "/opt/homebrew/bin/para",              // Apple Silicon
      "/usr/local/bin/para",                 // Intel Mac
      "/home/linuxbrew/.linuxbrew/bin/para", // Linux
    ];
    
    for (const location of homebrewLocations) {
      try {
        execSync(`test -x ${location}`, { stdio: 'ignore' });
        return location;
      } catch {
        // Continue to next location
      }
    }
    
    // If homebrew MCP but no homebrew para found, there's a problem
    console.error("Warning: Homebrew MCP server but para binary not found in homebrew locations");
  }
  
  // For development or other installations, check in order
  const locations = [
    process.cwd() + "/target/release/para",           // Local development build
    process.cwd() + "/target/debug/para",             // Local debug build
    process.env.HOME + "/.local/bin/para",           // Local installation
    "/opt/homebrew/bin/para",                        // Homebrew fallback
    "/usr/local/bin/para",                           // Homebrew fallback
    "para"                                           // System PATH
  ];

  for (const location of locations) {
    try {
      execSync(`test -x ${location}`, { stdio: 'ignore' });
      return location;
    } catch {
      // Continue to next location
    }
  }

  // Fallback to 'para' in PATH
  return "para";
}

const PARA_BINARY = findParaBinary();
console.error(`Para MCP server using para binary: ${PARA_BINARY}`);

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
  return new Promise((resolve, reject) => {
    try {
      // Properly quote arguments that contain spaces
      const quotedArgs = args.map(arg => {
        // If the argument contains spaces and isn't already quoted, wrap it in quotes
        if (arg.includes(' ') && !arg.startsWith('"') && !arg.startsWith("'")) {
          return `"${arg.replace(/"/g, '\\"')}"`;
        }
        return arg;
      });

      const command = `${PARA_BINARY} ${quotedArgs.join(' ')}`;
      
      // Set up environment to indicate non-interactive mode
      const env = {
        ...process.env,
        PARA_NON_INTERACTIVE: '1',
        CI: '1'  // Many CLIs respect this as non-interactive indicator
      };
      
      const child = exec(command, { env }, (error, stdout, stderr) => {
        if (error) {
          reject(new McpError(ErrorCode.InternalError, `Para command failed: ${error.message}`));
          return;
        }
        
        if (stderr && !stderr.includes('warning')) {
          console.error(`Para command warning: ${stderr}`);
        }
        
        resolve(stdout.trim());
      });
      
      // Set up timeout of 30 seconds
      const timeout = setTimeout(() => {
        child.kill();
        reject(new McpError(
          ErrorCode.InternalError, 
          `Command timed out after 30 seconds. The command may be waiting for interactive input which is not supported in MCP mode.`
        ));
      }, 30000);
      
      // Clear timeout if command completes
      child.on('exit', () => {
        clearTimeout(timeout);
      });
      
    } catch (error: any) {
      reject(new McpError(ErrorCode.InternalError, `Para command failed: ${error.message}`));
    }
  });
}

// List available tools
server.setRequestHandler(ListToolsRequestSchema, async () => {
  return {
    tools: [
      {
        name: "para_start",
        description: "Start manual development session in isolated Git worktree. For complex tasks where YOU (orchestrator) work WITH the user, not for dispatching agents. Creates .para/worktrees/session-name directory. Use when user needs direct involvement or task is too complex for agents.",
        inputSchema: {
          type: "object",
          properties: {
            session_name: {
              type: "string",
              description: "Name for the new session (optional, generates friendly name if not provided)"
            },
            dangerously_skip_permissions: {
              type: "boolean",
              description: "Skip IDE permission warnings (dangerous)"
            }
          },
          required: []
        }
      },
      {
        name: "para_finish",
        description: "Rarely used by orchestrator. Creates branch without merging. Agents use CLI 'para finish' command instead. Only use if you started a manual session with para_start and want to save work without merging.",
        inputSchema: {
          type: "object",
          properties: {
            commit_message: {
              type: "string",
              description: "Commit message describing the changes made"
            },
            session: {
              type: "string",
              description: "Session ID (optional, auto-detects if not provided)"
            },
            branch: {
              type: "string",
              description: "Custom branch name instead of default para/session-name. If branch already exists, error with suggestion."
            }
          },
          required: ["commit_message"]
        }
      },
      {
        name: "para_dispatch",
        description: "PRIMARY TOOL: Dispatch AI agents for parallel development. Each agent works in isolated Git worktree.\n\nPARALLELIZATION:\n- SEQUENTIAL: API spec first → then implementations\n- PARALLEL: Frontend + Backend (using same API)\n- AVOID: Same files = conflicts\n\nTASK FORMAT:\n- PREFER FILE: Use task files for complex prompts or special characters\n- INLINE ONLY: Simple, short natural language tasks without special symbols\n- DEFAULT: Create .md file in 'tasks/' directory (recommended)\n\nTASK WRITING:\n- Keep simple, avoid overengineering\n- State WHAT not HOW\n- Let agents choose implementation\n- End with: 'When done: para finish \"<msg>\"'\n- CUSTOM BRANCHES: Add '--branch custom-name' for specific branch names\n\nWORKFLOW:\n1. Create tasks/TASK_1_feature.md files\n2. Dispatch agents (they'll finish work automatically)\n3. Continue with user on next tasks\n4. Conflicts? Review branches manually with user\n\nEXAMPLE TASK:\n```\nImplement user authentication with email/password.\nStore users in database.\nReturn JWT tokens.\n\nWhen done: para finish \"Add user authentication\" --branch feature/auth-system\n```",
        inputSchema: {
          type: "object",
          properties: {
            session_name: {
              type: "string",
              description: "Unique name for this agent/session (e.g., 'auth-api', 'frontend-ui')"
            },
            task_description: {
              type: "string",
              description: "Inline task description for SIMPLE tasks only. Use for short, natural language prompts without special characters. Must end with workflow instruction: 'When complete, run: para finish \"<commit msg>\"'"
            },
            file: {
              type: "string",
              description: "Path to task file (e.g., tasks/TASK_1_auth.md). Default directory: tasks/"
            },
            dangerously_skip_permissions: {
              type: "boolean",
              description: "Skip IDE permission warnings (dangerous)"
            }
          },
          required: ["session_name"]
        }
      },
      {
        name: "para_list",
        description: "Check status if needed. Shows sessions/agents. Not required - focus on dispatching agents and working with user. Agents handle their own integration.",
        inputSchema: {
          type: "object",
          properties: {
            verbose: {
              type: "boolean",
              description: "Show detailed session information including paths and timestamps"
            },
            archived: {
              type: "boolean",
              description: "Include finished/archived sessions in the list"
            },
            quiet: {
              type: "boolean",
              description: "Minimal output for scripts"
            }
          },
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
              description: "Name of the session to recover (optional, shows list if not provided)"
            }
          },
          required: []
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
        description: "DESTRUCTIVE: Permanently delete a para session, removing its worktree and branch. All uncommitted work will be lost. WARNING: Never use this on your current session - it will delete all your work! Use para_finish or para_recover instead. Only use this to clean up abandoned sessions.",
        inputSchema: {
          type: "object",
          properties: {
            session_name: {
              type: "string",
              description: "Name of the session to cancel (optional, auto-detects current session - DANGEROUS!)"
            }
          },
          required: []
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
        {
          const cmdArgs = ["start"];
          if ((args as any).session_name) {
            cmdArgs.push((args as any).session_name);
          }
          if ((args as any).dangerously_skip_permissions) {
            cmdArgs.push("--dangerously-skip-permissions");
          }
          result = await runParaCommand(cmdArgs);
        }
        break;

      case "para_finish":
        {
          const cmdArgs = ["finish"];
          cmdArgs.push((args as any).commit_message);
          if ((args as any).session) {
            cmdArgs.push((args as any).session);
          }
          if ((args as any).branch) {
            cmdArgs.push("--branch", (args as any).branch);
          }
          result = await runParaCommand(cmdArgs);
        }
        break;

      case "para_dispatch":
        {
          const cmdArgs = ["dispatch"];
          cmdArgs.push((args as any).session_name);

          if ((args as any).file) {
            cmdArgs.push("--file", (args as any).file);
          } else if ((args as any).task_description) {
            cmdArgs.push((args as any).task_description);
          }

          if ((args as any).dangerously_skip_permissions) {
            cmdArgs.push("--dangerously-skip-permissions");
          }

          result = await runParaCommand(cmdArgs);
        }
        break;

      case "para_list":
        {
          const cmdArgs = ["list"];
          if ((args as any).verbose) {
            cmdArgs.push("--verbose");
          }
          if ((args as any).archived) {
            cmdArgs.push("--archived");
          }
          if ((args as any).quiet) {
            cmdArgs.push("--quiet");
          }
          result = await runParaCommand(cmdArgs);
        }
        break;

      case "para_recover":
        {
          const cmdArgs = ["recover"];
          if ((args as any).session_name) {
            cmdArgs.push((args as any).session_name);
          }
          result = await runParaCommand(cmdArgs);
        }
        break;

      case "para_config_show":
        result = await runParaCommand(["config", "show"]);
        break;

      case "para_cancel":
        {
          const cmdArgs = ["cancel"];
          if ((args as any).session_name) {
            cmdArgs.push((args as any).session_name);
          }
          result = await runParaCommand(cmdArgs);
        }
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