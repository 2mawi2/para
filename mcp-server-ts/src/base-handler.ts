import { McpError, ErrorCode } from "@modelcontextprotocol/sdk/types.js";
import { ToolDefinition, ToolResult } from "./types.js";

export interface CommandHandler {
  getToolDefinition(): ToolDefinition;
  execute(args: Record<string, unknown>): Promise<ToolResult>;
  validateArgs(args: Record<string, unknown>): void;
}

export abstract class BaseCommandHandler implements CommandHandler {
  protected validateRequiredArgs(args: Record<string, unknown>, required: string[]): void {
    for (const field of required) {
      if (args[field] === undefined || args[field] === null) {
        throw new McpError(
          ErrorCode.InvalidParams,
          `Missing required parameter: ${field}`
        );
      }
    }
  }

  protected validateArgTypes(args: Record<string, unknown>, schema: Record<string, string>): void {
    for (const [field, expectedType] of Object.entries(schema)) {
      if (args[field] !== undefined) {
        const actualType = typeof args[field];
        if (actualType !== expectedType) {
          throw new McpError(
            ErrorCode.InvalidParams,
            `Parameter ${field} must be of type ${expectedType}, got ${actualType}`
          );
        }
      }
    }
  }

  abstract getToolDefinition(): ToolDefinition;
  abstract execute(args: Record<string, unknown>): Promise<ToolResult>;
  abstract validateArgs(args: Record<string, unknown>): void;
}