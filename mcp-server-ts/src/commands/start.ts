import { ParaStartArgs } from "../types/command-args.js";
import { runParaCommand } from "../utils/command-executor.js";

/**
 * Handle para_start command
 */
export async function handleParaStart(args: ParaStartArgs, paraBinary: string): Promise<string> {
  const cmdArgs = ["start"];
  
  if (args.session_name) {
    cmdArgs.push(args.session_name);
  }
  
  if (args.dangerously_skip_permissions) {
    cmdArgs.push("--dangerously-skip-permissions");
  }
  
  return await runParaCommand(cmdArgs, paraBinary);
}

/**
 * MCP tool definition for para_start
 */
export const paraStartTool = {
  name: "para_start",
  description: "Start manual development session in isolated Git worktree. For complex tasks where YOU (orchestrator) work WITH the user, not for dispatching agents. Creates .para/worktrees/session-name directory. Use when user needs direct involvement or task is too complex for agents.",
  inputSchema: {
    type: "object",
    properties: {
      session_name: {
        type: "string",
        description: "Name for the new session (optional, generates friendly name if not provided)"
      },
      dangerously_skip_permissions: {
        type: "boolean",
        description: "Skip IDE permission warnings (dangerous)"
      }
    },
    required: []
  }
} as const;