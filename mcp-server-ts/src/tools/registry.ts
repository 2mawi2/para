/**
 * Tool Registry - Central registry for all para MCP tools
 */

import { McpError, ErrorCode } from "@modelcontextprotocol/sdk/types.js";

export interface ToolDefinition {
  name: string;
  description: string;
  inputSchema: {
    type: string;
    properties: Record<string, any>;
    required: string[];
    additionalProperties?: boolean;
  };
}

export interface ToolCall {
  name: string;
  arguments: Record<string, any>;
}

export class ToolRegistry {
  private tools: Map<string, ToolDefinition> = new Map();

  constructor() {
    this.registerParaTools();
  }

  /**
   * Register all para tools with their definitions
   */
  private registerParaTools(): void {
    const toolDefinitions: ToolDefinition[] = [
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
          additionalProperties: false,
          required: []
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
          additionalProperties: false,
          required: []
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
      },
      {
        name: "para_status_show",
        description: "Monitor agent progress across para sessions. Get agent-reported status including current task, test results (whole codebase health), confidence levels, todo progress, and blocked status. Use this to coordinate parallel development and identify agents needing assistance.\n\nOUTPUT INCLUDES:\n- current_task: What the agent is currently working on\n- test_status: passed/failed/unknown (reflects ALL tests in codebase)\n- confidence: high/medium/low (agent's self-assessment)\n- is_blocked: Whether agent needs help\n- todo_percentage: Progress through tasks\n- last_update: When status was last reported\n\nORCHESTRATOR USAGE:\n- Monitor all agents: para_status_show()\n- Check specific agent: para_status_show(session: 'agent-name')\n- Get structured data: para_status_show(json: true)\n\nREAD-ONLY: This tool only reads status. Agents update their own status via CLI.",
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
          additionalProperties: false,
          required: []
        }
      }
    ];

    // Register all tools
    toolDefinitions.forEach(tool => {
      this.tools.set(tool.name, tool);
    });
  }

  /**
   * Get all registered tool definitions
   */
  public getAllToolDefinitions(): ToolDefinition[] {
    return Array.from(this.tools.values());
  }

  /**
   * Get a specific tool definition by name
   */
  public getToolDefinition(name: string): ToolDefinition | undefined {
    return this.tools.get(name);
  }

  /**
   * Check if a tool exists
   */
  public hasToolName(name: string): boolean {
    return this.tools.has(name);
  }

  /**
   * Validate a tool call against its schema
   */
  public validateToolCall(name: string, args: Record<string, any>): boolean {
    const tool = this.tools.get(name);
    if (!tool) {
      throw new McpError(ErrorCode.MethodNotFound, `Unknown tool: ${name}`);
    }

    // Check required parameters
    for (const requiredParam of tool.inputSchema.required) {
      if (!(requiredParam in args)) {
        throw new McpError(ErrorCode.InvalidParams, `Missing required parameter: ${requiredParam}`);
      }
    }

    // Basic type validation could be added here
    // For now, we'll rely on the MCP SDK's built-in validation

    return true;
  }

  /**
   * Get the list of tool names
   */
  public getToolNames(): string[] {
    return Array.from(this.tools.keys());
  }
}