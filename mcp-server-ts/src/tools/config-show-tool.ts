/**
 * Para config show tool implementation
 */

import { ToolDefinition } from '../types/mcp-types.js';
import { ParaExecutor } from '../execution/para-executor.js';

export const configShowToolDefinition: ToolDefinition = {
  name: "para_config_show",
  description: "Display current para configuration including IDE, directories, and Git settings.",
  inputSchema: {
    type: "object",
    properties: {},
    additionalProperties: false
  }
};

export async function executeConfigShowTool(executor: ParaExecutor): Promise<string> {
  return executor.runCommand(["config", "show"]);
}