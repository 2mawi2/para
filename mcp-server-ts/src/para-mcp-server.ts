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
  name?: string;
  prompt?: string;
  file?: string;
  dangerously_skip_permissions?: boolean;
  container?: boolean;
  docker_args?: string[];
  sandbox?: boolean;
  no_sandbox?: boolean;
  sandbox_profile?: string;
  sandbox_no_network?: boolean;
  docker_image?: string;
  allow_domains?: string;
  no_forward_keys?: boolean;
  setup_script?: string;
}

interface ParaFinishArgs {
  commit_message: string;
  session?: string;
  branch?: string;
}

interface ParaListArgs {
  verbose?: boolean;
  archived?: boolean;
  quiet?: boolean;
}

interface ParaResumeArgs {
  session?: string;
  prompt?: string;
  file?: string;
  dangerously_skip_permissions?: boolean;
  sandbox?: boolean;
  no_sandbox?: boolean;
  sandbox_profile?: string;
  sandbox_no_network?: boolean;
}

interface ParaRecoverArgs {
  session_name?: string;
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
        description: "Start NEW para sessions. This tool creates fresh isolated Git worktrees for new development work.\n\n🎯 CORE PURPOSE: Start new sessions for AI agents or interactive development\n\n📋 USAGE PATTERNS:\n1. AI AGENT SESSION: para_start(prompt: \"implement user authentication\")\n2. AI FROM FILE: para_start(file: \"tasks/auth_requirements.md\") \n3. NAMED AI SESSION: para_start(name: \"auth-feature\", prompt: \"add JWT tokens\")\n4. INTERACTIVE SESSION: para_start(name: \"my-feature\") or para_start()\n\n📁 FILE INPUT: Use 'file' parameter to read complex requirements, specifications, or task descriptions from files. Files can contain:\n- Technical specifications\n- Code examples\n- Multi-step instructions  \n- Project requirements\n\n🔒 SANDBOXING FOR AUTONOMOUS AGENTS:\n1. BASIC SANDBOX (macOS): para_start(prompt: \"task\", dangerously_skip_permissions: true, sandbox: true)\n   - Restricts file writes to session directory only\n   - Full network access allowed\n   - Protects against prompt injection modifying system files\n\n2. NETWORK ISOLATION (macOS): para_start(prompt: \"task\", sandbox_no_network: true)\n   - Same file restrictions as basic sandbox\n   - Network limited to GitHub API only (via proxy)\n   - Optional: allow_domains: \"example.com,api.openai.com\" for additional domains\n\n3. DOCKER CONTAINER (all platforms): para_start(prompt: \"task\", container: true)\n   - Complete isolation from host system\n   - Only worktree mounted in container\n   - Optional: docker_image: \"ubuntu:22.04\" for custom images\n\n⚠️ IMPORTANT: This tool is ONLY for NEW sessions. If a session already exists, para_start will error and direct you to use para_resume instead.\n\n🔄 FOR EXISTING SESSIONS: Use para_resume to continue work on existing sessions with additional context or follow-up tasks.",
        inputSchema: {
          type: "object",
          properties: {
            name: {
              type: "string",
              description: "Name for the NEW session (e.g., 'auth-feature', 'payment-api'). Must contain only alphanumeric characters, hyphens, and underscores. If omitted, para generates a unique name."
            },
            prompt: {
              type: "string",
              description: "Task description or additional context (triggers AI agent mode)"
            },
            file: {
              type: "string",
              description: "File containing task description, requirements, or specifications for the NEW session (e.g., 'tasks/auth-requirements.md', 'specs/api-design.txt', 'prompts/implement-feature.md')"
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
              description: "Additional Docker arguments"
            },
            sandbox: {
              type: "boolean",
              description: "Enable sandboxing (macOS only) - restricts file writes to session directory while allowing reads. Protects against prompt injection attacks."
            },
            no_sandbox: {
              type: "boolean",
              description: "Disable sandboxing (overrides config)"
            },
            sandbox_profile: {
              type: "string",
              description: "Sandbox profile: 'standard' (default - full network) or 'standard-proxied' (GitHub API only via proxy)"
            },
            sandbox_no_network: {
              type: "boolean",
              description: "Enable network-isolated sandboxing (macOS only) - restricts network access to GitHub API only via proxy. Includes all file write restrictions from basic sandbox."
            },
            docker_image: {
              type: "string",
              description: "Custom Docker image (e.g., 'ubuntu:22.04')"
            },
            allow_domains: {
              type: "string",
              description: "Enable network isolation with allowed domains (comma-separated)"
            },
            no_forward_keys: {
              type: "boolean",
              description: "Disable automatic API key forwarding to Docker containers"
            },
            setup_script: {
              type: "string",
              description: "Path to setup script to run after session creation"
            }
          },
          required: []
        }
      },
      {
        name: "para_finish",
        description: "Complete session and create feature branch for review. Creates commit and branch from session work. Agents typically use CLI 'para finish' command instead.",
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
        name: "para_resume",
        description: "Resume EXISTING para sessions with additional context or follow-up tasks. This tool continues work on sessions that were previously started.\n\n🎯 CORE PURPOSE: Continue existing sessions with new instructions, requirements, or context\n\n📋 USAGE PATTERNS:\n1. RESUME CURRENT: para_resume() → Detects and resumes session in current directory\n2. RESUME SPECIFIC: para_resume(session: \"auth-feature\") → Resume named session\n3. ADD NEW TASK: para_resume(session: \"auth\", prompt: \"add password reset functionality\")\n4. NEW REQUIREMENTS: para_resume(session: \"api\", file: \"additional-requirements.md\")\n\n📁 FILE INPUT: Use 'file' parameter to provide:\n- Follow-up requirements or specifications\n- Additional tasks or features to implement\n- Updated technical requirements\n- Bug reports or fixes needed\n- New user stories or acceptance criteria\n\n💡 KEY BENEFIT: Perfect for iterative development where you want to add more functionality or address new requirements in an existing session without starting over.\n\n⚠️ IMPORTANT: Session must already exist. If session doesn't exist, you'll get an error suggesting to use para_start instead.",
        inputSchema: {
          type: "object",
          properties: {
            session: {
              type: "string",
              description: "Session ID to resume (optional, auto-detects from current directory if not provided)"
            },
            prompt: {
              type: "string",
              description: "Additional instructions, new tasks, or follow-up requirements for the existing session (e.g., 'add validation to the login form', 'fix the bug in user registration')"
            },
            file: {
              type: "string",
              description: "File containing follow-up tasks, additional requirements, or new specifications to add to the existing session (e.g., 'tasks/phase2-requirements.md', 'bugs/login-issues.txt')"
            },
            dangerously_skip_permissions: {
              type: "boolean",
              description: "Skip IDE permission warnings (DANGEROUS: Only use for automated scripts)"
            },
            sandbox: {
              type: "boolean",
              description: "Enable sandboxing (macOS only) - restricts file writes to session directory while allowing reads. Protects against prompt injection attacks."
            },
            no_sandbox: {
              type: "boolean",
              description: "Disable sandboxing (overrides config)"
            },
            sandbox_profile: {
              type: "string",
              description: "Sandbox profile: 'standard' (default - full network) or 'standard-proxied' (GitHub API only via proxy)"
            },
            sandbox_no_network: {
              type: "boolean",
              description: "Enable network-isolated sandboxing (macOS only) - restricts network access to GitHub API only via proxy. Includes all file write restrictions from basic sandbox."
            }
          },
          required: []
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
          
          // Handle new argument structure
          if (startArgs.name) {
            cmdArgs.push(startArgs.name);
          }
          if (startArgs.prompt) {
            cmdArgs.push("-p", startArgs.prompt);
          }
          if (startArgs.file) {
            cmdArgs.push("--file", startArgs.file);
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
          if (startArgs.sandbox) {
            cmdArgs.push("--sandbox");
          }
          if (startArgs.no_sandbox) {
            cmdArgs.push("--no-sandbox");
          }
          if (startArgs.sandbox_profile) {
            cmdArgs.push("--sandbox-profile", startArgs.sandbox_profile);
          }
          if (startArgs.sandbox_no_network) {
            cmdArgs.push("--sandbox-no-network");
          }
          if (startArgs.docker_image) {
            cmdArgs.push("--docker-image", startArgs.docker_image);
          }
          if (startArgs.allow_domains) {
            cmdArgs.push("--allow-domains", startArgs.allow_domains);
          }
          if (startArgs.no_forward_keys) {
            cmdArgs.push("--no-forward-keys");
          }
          if (startArgs.setup_script) {
            cmdArgs.push("--setup-script", startArgs.setup_script);
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
          if (resumeArgs.dangerously_skip_permissions) {
            cmdArgs.push("--dangerously-skip-permissions");
          }
          if (resumeArgs.sandbox) {
            cmdArgs.push("--sandbox");
          }
          if (resumeArgs.no_sandbox) {
            cmdArgs.push("--no-sandbox");
          }
          if (resumeArgs.sandbox_profile) {
            cmdArgs.push("--sandbox-profile", resumeArgs.sandbox_profile);
          }
          if (resumeArgs.sandbox_no_network) {
            cmdArgs.push("--sandbox-no-network");
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