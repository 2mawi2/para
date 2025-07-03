import { ParaListArgs } from "../types/command-args.js";
import { runParaCommand } from "../utils/command-executor.js";

/**
 * Handle para_list command
 */
export async function handleParaList(args: ParaListArgs, paraBinary: string): Promise<string> {
  const cmdArgs = ["list"];
  
  if (args.verbose) {
    cmdArgs.push("--verbose");
  }
  
  if (args.archived) {
    cmdArgs.push("--archived");
  }
  
  if (args.quiet) {
    cmdArgs.push("--quiet");
  }
  
  return await runParaCommand(cmdArgs, paraBinary);
}

/**
 * MCP tool definition for para_list
 */
export const paraListTool = {
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
} as const;