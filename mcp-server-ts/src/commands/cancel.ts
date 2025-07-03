import { ParaCancelArgs } from "../types/command-args.js";
import { runParaCommand } from "../utils/command-executor.js";

/**
 * Handle para_cancel command
 */
export async function handleParaCancel(args: ParaCancelArgs, paraBinary: string): Promise<string> {
  const cmdArgs = ["cancel"];
  
  if (args.session_name) {
    cmdArgs.push(args.session_name);
  }
  
  if (args.force) {
    cmdArgs.push("--force");
  }
  
  return await runParaCommand(cmdArgs, paraBinary);
}

/**
 * MCP tool definition for para_cancel
 */
export const paraCancelTool = {
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
} as const;