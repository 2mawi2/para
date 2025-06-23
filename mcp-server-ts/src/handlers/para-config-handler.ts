import { BaseCommandHandler } from "../base-handler.js";
import { ToolDefinition, ToolResult } from "../types.js";
import { runParaCommand } from "../para-utils.js";

export class ParaConfigHandler extends BaseCommandHandler {
  constructor(private paraBinary: string) {
    super();
  }

  getToolDefinition(): ToolDefinition {
    return {
      name: "para_config_show",
      description: "Display current para configuration including IDE, directories, and Git settings.",
      inputSchema: {
        type: "object",
        properties: {},
        additionalProperties: false
      }
    };
  }

  validateArgs(_args: Record<string, unknown>): void {
    // No validation needed - empty schema
  }

  async execute(_args: Record<string, unknown>): Promise<ToolResult> {
    const result = await runParaCommand(["config", "show"], this.paraBinary);
    
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