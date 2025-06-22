/**
 * Management Tools
 * 
 * MCP tool definitions for para management operations:
 * list, status, config, cancel
 */

export const managementTools = [
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
      additionalProperties: false
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
        },
        force: {
          type: "boolean",
          description: "Force cancellation without confirmation prompts"
        }
      },
      required: []
    }
  }
];

export type ManagementToolName = "para_list" | "para_status_show" | "para_config_show" | "para_cancel";