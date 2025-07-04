/**
 * Para recover tool implementation
 */

import { ToolDefinition } from '../types/mcp-types.js';
import { ParaRecoverArgs } from '../types/para-args.js';
import { ParaExecutor } from '../execution/para-executor.js';

export const recoverToolDefinition: ToolDefinition = {
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

export async function executeRecoverTool(args: ParaRecoverArgs, executor: ParaExecutor): Promise<string> {
  const cmdArgs = ["recover"];
  
  if (args.session_name) {
    cmdArgs.push(args.session_name);
  }
  
  return executor.runCommand(cmdArgs);
}