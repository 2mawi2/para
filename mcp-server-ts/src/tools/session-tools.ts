/**
 * Session Management Tools
 * 
 * MCP tool definitions for session-related operations:
 * start, finish, resume, recover
 */

export const sessionTools = [
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
  }
];

export type SessionToolName = "para_start" | "para_finish" | "para_resume" | "para_recover";