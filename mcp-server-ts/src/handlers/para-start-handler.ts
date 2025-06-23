import { BaseCommandHandler } from "../base-handler.js";
import { ToolDefinition, ToolResult, ParaStartArgs } from "../types.js";
import { runParaCommand } from "../para-utils.js";

export class ParaStartHandler extends BaseCommandHandler {
  constructor(private readonly paraBinary: string) {
    super();
  }

  getToolDefinition(): ToolDefinition {
    return {
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
    };
  }

  validateArgs(args: Record<string, unknown>): void {
    this.validateArgTypes(args, {
      session_name: "string",
      dangerously_skip_permissions: "boolean"
    });
  }

  async execute(args: Record<string, unknown>): Promise<ToolResult> {
    this.validateArgs(args);
    const startArgs = args as ParaStartArgs;
    
    const cmdArgs = ["start"];
    if (startArgs.session_name) {
      cmdArgs.push(startArgs.session_name);
    }
    if (startArgs.dangerously_skip_permissions) {
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