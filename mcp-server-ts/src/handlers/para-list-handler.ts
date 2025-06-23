import { BaseCommandHandler } from "../base-handler.js";
import { ToolDefinition, ToolResult, ParaListArgs } from "../types.js";
import { runParaCommand } from "../para-utils.js";

export class ParaListHandler extends BaseCommandHandler {
  constructor(private paraBinary: string) {
    super();
  }

  getToolDefinition(): ToolDefinition {
    return {
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
    };
  }

  validateArgs(args: Record<string, unknown>): void {
    this.validateArgTypes(args, {
      verbose: "boolean",
      archived: "boolean",
      quiet: "boolean"
    });
  }

  async execute(args: Record<string, unknown>): Promise<ToolResult> {
    this.validateArgs(args);
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