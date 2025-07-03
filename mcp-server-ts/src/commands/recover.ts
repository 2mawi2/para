import { ParaRecoverArgs } from "../types/command-args.js";
import { runParaCommand } from "../utils/command-executor.js";

/**
 * Handle para_recover command
 */
export async function handleParaRecover(args: ParaRecoverArgs, paraBinary: string): Promise<string> {
  const cmdArgs = ["recover"];
  
  if (args.session_name) {
    cmdArgs.push(args.session_name);
  }
  
  return await runParaCommand(cmdArgs, paraBinary);
}

/**
 * MCP tool definition for para_recover
 */
export const paraRecoverTool = {
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
} as const;