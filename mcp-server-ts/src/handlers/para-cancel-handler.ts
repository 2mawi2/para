import { BaseCommandHandler } from "../base-handler.js";
import { ToolDefinition, ToolResult, ParaCancelArgs } from "../types.js";
import { runParaCommand } from "../para-utils.js";

export class ParaCancelHandler extends BaseCommandHandler {
  constructor(private paraBinary: string) {
    super();
  }

  getToolDefinition(): ToolDefinition {
    return {
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
    };
  }

  validateArgs(args: Record<string, unknown>): void {
    this.validateArgTypes(args, {
      session_name: "string",
      force: "boolean"
    });
  }

  async execute(args: Record<string, unknown>): Promise<ToolResult> {
    this.validateArgs(args);
    const cancelArgs = args as ParaCancelArgs;
    
    const cmdArgs = ["cancel"];
    if (cancelArgs.session_name) {
      cmdArgs.push(cancelArgs.session_name);
    }
    if (cancelArgs.force) {
      cmdArgs.push("--force");
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