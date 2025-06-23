import { BaseCommandHandler } from "../base-handler.js";
import { ToolDefinition, ToolResult, ParaDispatchArgs } from "../types.js";
import { runParaCommand } from "../para-utils.js";

export class ParaDispatchHandler extends BaseCommandHandler {
  constructor(private paraBinary: string) {
    super();
  }

  getToolDefinition(): ToolDefinition {
    return {
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
    };
  }

  validateArgs(args: Record<string, unknown>): void {
    this.validateRequiredArgs(args, ["session_name"]);
    this.validateArgTypes(args, {
      session_name: "string",
      task_description: "string",
      file: "string",
      dangerously_skip_permissions: "boolean"
    });
  }

  async execute(args: Record<string, unknown>): Promise<ToolResult> {
    this.validateArgs(args);
    const dispatchArgs = args as ParaDispatchArgs;
    
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
    
    const result = await runParaCommand(cmdArgs, this.paraBinary);
    
    return {
      content: [
        {
          type: "text",
          text: result
        }
      ]
    };
  }
}