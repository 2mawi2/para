export const PARA_TOOLS = [
  {
    name: "para_start",
    description: "Start NEW para sessions. This tool creates fresh isolated Git worktrees for new development work.\n\n🎯 CORE PURPOSE: Start new sessions for AI agents or interactive development\n\n📋 USAGE PATTERNS:\n1. AI AGENT SESSION: para_start(prompt: \"implement user authentication\")\n2. AI FROM FILE: para_start(file: \"tasks/auth_requirements.md\") \n3. NAMED AI SESSION: para_start(name_or_session: \"auth-feature\", prompt: \"add JWT tokens\")\n4. INTERACTIVE SESSION: para_start(name_or_session: \"my-feature\") or para_start()\n\n📁 FILE INPUT: Use 'file' parameter to read complex requirements, specifications, or task descriptions from files. Files can contain:\n- Technical specifications\n- Code examples\n- Multi-step instructions  \n- Project requirements\n\n⚠️ IMPORTANT: This tool is ONLY for NEW sessions. If a session already exists, para_start will error and direct you to use para_resume instead.\n\n🔄 FOR EXISTING SESSIONS: Use para_resume to continue work on existing sessions with additional context or follow-up tasks.",
    inputSchema: {
      type: "object",
      properties: {
        name_or_session: {
          type: "string",
          description: "Name for the NEW session (e.g., 'auth-feature', 'payment-api'). If omitted, para generates a unique name."
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
          description: "Enable sandboxing (overrides config)"
        },
        no_sandbox: {
          type: "boolean",
          description: "Disable sandboxing (overrides config)"
        },
        sandbox_profile: {
          type: "string",
          description: "Sandbox profile: permissive (default) or restrictive"
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
          description: "Enable sandboxing (overrides config)"
        },
        no_sandbox: {
          type: "boolean",
          description: "Disable sandboxing (overrides config)"
        },
        sandbox_profile: {
          type: "string",
          description: "Sandbox profile: permissive (default) or restrictive"
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
];