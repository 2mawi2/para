/**
 * Para list tool implementation
 */

import { ToolDefinition } from '../types/mcp-types.js';
import { ParaListArgs } from '../types/para-args.js';
import { ParaExecutor } from '../execution/para-executor.js';

export const listToolDefinition: ToolDefinition = {
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

export async function executeListTool(args: ParaListArgs, executor: ParaExecutor): Promise<string> {
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
  
  return executor.runCommand(cmdArgs);
}