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

interface ParaStartArgs {
  session_name?: string;
  dangerously_skip_permissions?: boolean;
  container?: boolean;
  docker_args?: string[];
}

interface ParaFinishArgs {
  commit_message: string;
  session?: string;
  branch?: string;
}

interface ParaDispatchArgs {
  session_name: string;
  task_description?: string;
  file?: string;
  dangerously_skip_permissions?: boolean;
  container?: boolean;
  docker_args?: string[];
}

interface ParaListArgs {
  verbose?: boolean;
  archived?: boolean;
  quiet?: boolean;
}

interface ParaRecoverArgs {
  session_name?: string;
}

interface ParaResumeArgs {
  session?: string;
  prompt?: string;
  file?: string;
}

interface ParaCancelArgs {
  session_name?: string;
  force?: boolean;
}

interface ParaStatusShowArgs {
  session?: string;
  json?: boolean;
}

interface ParaConfigSetArgs {
  path: string;
  value: string | boolean | number;
}

// Dynamic discovery needed to support homebrew, dev builds, and system installations
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

// Abstracts command execution to handle timeouts, environment setup, and error handling
async function runParaCommand(args: string[]): Promise<string> {
  return new Promise((resolve, reject) => {
    // Prevent shell injection and argument splitting issues
    const quotedArgs = args.map(arg => {
      if (arg.includes(' ') && !arg.startsWith('"') && !arg.startsWith("'")) {
        return `"${arg.replace(/"/g, '\\"')}"`;
      }
      return arg;
    });

    const command = `${PARA_BINARY} ${quotedArgs.join(' ')}`;
    
    // Prevent para from blocking on user prompts in automated contexts
    const env = {
      ...process.env,
      PARA_NON_INTERACTIVE: '1',
      CI: '1'  // Many CLIs respect this as well
    };

    const child = exec(command, { env }, (error, stdout, stderr) => {
      clearTimeout(timeout);
      
      if (error) {
        reject(new McpError(ErrorCode.InternalError, `Para command failed: ${error.message}`));
        return;
      }
      
      if (stderr && !stderr.includes('warning')) {
        console.error(`Para command warning: ${stderr}`);
      }
      
      resolve(stdout.trim());
    });

    // Prevent hanging on complex operations while allowing time for worktree setup
    const timeout = setTimeout(() => {
      child.kill();
      reject(new McpError(ErrorCode.InternalError, `Command timed out after 30 seconds: ${args.join(' ')}`));
    }, 30000);
  });
}

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
            },
            container: {
              type: "boolean",
              description: "Run session in Docker container"
            },
            docker_args: {
              type: "array",
              items: { type: "string" },
              description: "Additional Docker arguments (e.g., ['-d'] for detached mode)"
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
            },
            container: {
              type: "boolean",
              description: "Run session in Docker container"
            },
            docker_args: {
              type: "array",
              items: { type: "string" },
              description: "Additional Docker arguments (e.g., ['-d'] for detached mode)"
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
        name: "para_resume",
        description: "Resume an existing active session with optional additional context or instructions. Opens the session's worktree in your IDE.",
        inputSchema: {
          type: "object",
          properties: {
            session: {
              type: "string",
              description: "Session ID to resume (optional, shows list if not provided)"
            },
            prompt: {
              type: "string",
              description: "Additional prompt or instructions for the resumed session"
            },
            file: {
              type: "string",
              description: "Read additional instructions from specified file"
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
        name: "para_config_set",
        description: "Set para configuration values using JSON path notation. Supports setting IDE, Git, directories, and session configuration. Values are automatically typed (string, boolean, number).",
        inputSchema: {
          type: "object",
          properties: {
            path: {
              type: "string",
              description: "JSON path using dot notation (e.g., 'ide.name', 'git.auto_stage', 'ide.wrapper.command', 'session.auto_cleanup_days')"
            },
            value: {
              oneOf: [
                { type: "string" },
                { type: "boolean" },
                { type: "number" }
              ],
              description: "Value to set - automatically typed based on input"
            }
          },
          required: ["path", "value"]
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
            },
            force: {
              type: "boolean",
              description: "Force cancellation without confirmation prompts"
            }
          },
          required: []
        }
      },
      {
        name: "para_status_show",
        description: "Monitor agent progress across para sessions. Get agent-reported status including current task, test results (whole codebase health), todo progress, and blocked status. Use this to coordinate parallel development and identify agents needing assistance.\n\nOUTPUT INCLUDES:\n- current_task: What the agent is currently working on\n- test_status: passed/failed/unknown (reflects ALL tests in codebase)\n- is_blocked: Whether agent needs help\n- todo_percentage: Progress through tasks\n- last_update: When status was last reported\n\nORCHESTRATOR USAGE:\n- Monitor all agents: para_status_show()\n- Check specific agent: para_status_show(session: 'agent-name')\n- Get structured data: para_status_show(json: true)\n\nREAD-ONLY: This tool only reads status. Agents update their own status via CLI.",
        inputSchema: {
          type: "object",
          properties: {
            session: {
              type: "string",
              description: "Session name to get status for (optional, shows all sessions if not provided)"
            },
            json: {
              type: "boolean",
              description: "Return structured JSON data instead of human-readable format",
              default: false
            }
          },
          additionalProperties: false
        }
      }
    ]
  };
});

server.setRequestHandler(CallToolRequestSchema, async (request) => {
  const { name, arguments: args } = request.params;

  try {
    let result: string;

    switch (name) {
      case "para_start":
        {
          const startArgs = args as ParaStartArgs;
          const cmdArgs = ["start"];
          if (startArgs.session_name) {
            cmdArgs.push(startArgs.session_name);
          }
          if (startArgs.dangerously_skip_permissions) {
            cmdArgs.push("--dangerously-skip-permissions");
          }
          if (startArgs.container) {
            cmdArgs.push("--container");
          }
          if (startArgs.docker_args && startArgs.docker_args.length > 0) {
            cmdArgs.push("--docker-args", ...startArgs.docker_args);
          }
          result = await runParaCommand(cmdArgs);
        }
        break;

      case "para_finish":
        {
          const finishArgs = args as unknown as ParaFinishArgs;
          const cmdArgs = ["finish"];
          cmdArgs.push(finishArgs.commit_message);
          if (finishArgs.session) {
            cmdArgs.push(finishArgs.session);
          }
          if (finishArgs.branch) {
            cmdArgs.push("--branch", finishArgs.branch);
          }
          result = await runParaCommand(cmdArgs);
        }
        break;

      case "para_dispatch":
        {
          const dispatchArgs = args as unknown as ParaDispatchArgs;
          const cmdArgs = ["dispatch"];
          cmdArgs.push(dispatchArgs.session_name);

          if (dispatchArgs.file) {
            cmdArgs.push("--file", dispatchArgs.file);
          } else if (dispatchArgs.task_description) {
            cmdArgs.push(dispatchArgs.task_description);
          }

          if (dispatchArgs.dangerously_skip_permissions) {
            cmdArgs.push("--dangerously-skip-permissions");
          }
          if (dispatchArgs.container) {
            cmdArgs.push("--container");
          }
          if (dispatchArgs.docker_args && dispatchArgs.docker_args.length > 0) {
            cmdArgs.push("--docker-args", ...dispatchArgs.docker_args);
          }

          result = await runParaCommand(cmdArgs);
        }
        break;

      case "para_list":
        {
          const listArgs = args as ParaListArgs;
          const cmdArgs = ["list"];
          if (listArgs.verbose) {
            cmdArgs.push("--verbose");
          }
          if (listArgs.archived) {
            cmdArgs.push("--archived");
          }
          if (listArgs.quiet) {
            cmdArgs.push("--quiet");
          }
          result = await runParaCommand(cmdArgs);
        }
        break;

      case "para_recover":
        {
          const recoverArgs = args as ParaRecoverArgs;
          const cmdArgs = ["recover"];
          if (recoverArgs.session_name) {
            cmdArgs.push(recoverArgs.session_name);
          }
          result = await runParaCommand(cmdArgs);
        }
        break;

      case "para_resume":
        {
          const resumeArgs = args as ParaResumeArgs;
          const cmdArgs = ["resume"];
          if (resumeArgs.session) {
            cmdArgs.push(resumeArgs.session);
          }
          if (resumeArgs.prompt) {
            cmdArgs.push("--prompt", resumeArgs.prompt);
          }
          if (resumeArgs.file) {
            cmdArgs.push("--file", resumeArgs.file);
          }
          result = await runParaCommand(cmdArgs);
        }
        break;

      case "para_config_show":
        result = await runParaCommand(["config", "show"]);
        break;

      case "para_config_set":
        {
          const configSetArgs = args as unknown as ParaConfigSetArgs;
          const cmdArgs = ["config", "set", configSetArgs.path, String(configSetArgs.value)];
          result = await runParaCommand(cmdArgs);
        }
        break;

      case "para_cancel":
        {
          const cancelArgs = args as ParaCancelArgs;
          const cmdArgs = ["cancel"];
          if (cancelArgs.session_name) {
            cmdArgs.push(cancelArgs.session_name);
          }
          if (cancelArgs.force) {
            cmdArgs.push("--force");
          }
          result = await runParaCommand(cmdArgs);
        }
        break;

      case "para_status_show":
        {
          const statusArgs = args as ParaStatusShowArgs;
          const cmdArgs = ["status", "show"];
          if (statusArgs.session) {
            cmdArgs.push(statusArgs.session);
          }
          if (statusArgs.json) {
            cmdArgs.push("--json");
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
  } catch (error: unknown) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    throw new McpError(ErrorCode.InternalError, `Tool execution failed: ${errorMessage}`);
  }
});

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
  } catch (error: unknown) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    throw new McpError(ErrorCode.InternalError, `Resource read failed: ${errorMessage}`);
  }
});

async function main(): Promise<void> {
  const transport = new StdioServerTransport();
  await server.connect(transport);
  console.error("Para MCP server running via TypeScript");
}

main().catch(console.error);