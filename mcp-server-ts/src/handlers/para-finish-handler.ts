import { BaseCommandHandler } from "../base-handler.js";
import { ToolDefinition, ToolResult, ParaFinishArgs } from "../types.js";
import { runParaCommand } from "../para-utils.js";

export class ParaFinishHandler extends BaseCommandHandler {
  constructor(private paraBinary: string) {
    super();
  }

  getToolDefinition(): ToolDefinition {
    return {
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
    };
  }

  validateArgs(args: Record<string, unknown>): void {
    this.validateRequiredArgs(args, ["commit_message"]);
    this.validateArgTypes(args, {
      commit_message: "string",
      session: "string",
      branch: "string"
    });
  }

  private parseFinishArgs(args: Record<string, unknown>): ParaFinishArgs {
    return {
      commit_message: args.commit_message as string,
      session: args.session as string | undefined,
      branch: args.branch as string | undefined
    };
  }

  async execute(args: Record<string, unknown>): Promise<ToolResult> {
    this.validateArgs(args);
    const finishArgs = this.parseFinishArgs(args);
    
    const cmdArgs = ["finish"];
    cmdArgs.push(finishArgs.commit_message);
    if (finishArgs.session) {
      cmdArgs.push(finishArgs.session);
    }
    if (finishArgs.branch) {
      cmdArgs.push("--branch", finishArgs.branch);
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