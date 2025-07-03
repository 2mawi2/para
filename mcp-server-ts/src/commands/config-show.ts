import { ParaConfigShowArgs } from "../types/command-args.js";
import { runParaCommand } from "../utils/command-executor.js";

/**
 * Handle para_config_show command
 */
export async function handleParaConfigShow(args: ParaConfigShowArgs, paraBinary: string): Promise<string> {
  return await runParaCommand(["config", "show"], paraBinary);
}

/**
 * MCP tool definition for para_config_show
 */
export const paraConfigShowTool = {
  name: "para_config_show",
  description: "Display current para configuration including IDE, directories, and Git settings.",
  inputSchema: {
    type: "object",
    properties: {},
    additionalProperties: false
  }
} as const;