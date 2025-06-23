import { BaseCommandHandler } from "../base-handler.js";
import { ToolDefinition, ToolResult, ParaRecoverArgs } from "../types.js";
import { runParaCommand } from "../para-utils.js";

export class ParaRecoverHandler extends BaseCommandHandler {
  constructor(private paraBinary: string) {
    super();
  }

  getToolDefinition(): ToolDefinition {
    return {
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
    };
  }

  validateArgs(args: Record<string, unknown>): void {
    this.validateArgTypes(args, {
      session_name: "string"
    });
  }

  async execute(args: Record<string, unknown>): Promise<ToolResult> {
    this.validateArgs(args);
    const recoverArgs = args as ParaRecoverArgs;
    
    const cmdArgs = ["recover"];
    if (recoverArgs.session_name) {
      cmdArgs.push(recoverArgs.session_name);
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