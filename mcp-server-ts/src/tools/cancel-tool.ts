/**
 * Para cancel tool implementation
 */

import { ToolDefinition } from '../types/mcp-types.js';
import { ParaCancelArgs } from '../types/para-args.js';
import { ParaExecutor } from '../execution/para-executor.js';

export const cancelToolDefinition: ToolDefinition = {
  name: "para_cancel",
  description: "DESTRUCTIVE: Permanently delete a para session, removing its worktree and branch. All uncommitted work will be lost. WARNING: Never use this on your current session - it will delete all your work! Use para_finish or para_recover instead. Only use this to clean up abandoned sessions.",
  inputSchema: {
    type: "object",
    properties: {
      session_name: {
        type: "string",
        description: "Name of the session to cancel (optional, auto-detects current session - DANGEROUS!)"
      },
      force: {
        type: "boolean",
        description: "Force cancellation without confirmation prompts"
      }
    },
    required: []
  }
};

export async function executeCancelTool(args: ParaCancelArgs, executor: ParaExecutor): Promise<string> {
  const cmdArgs = ["cancel"];
  
  if (args.session_name) {
    cmdArgs.push(args.session_name);
  }
  if (args.force) {
    cmdArgs.push("--force");
  }
  
  return executor.runCommand(cmdArgs);
}