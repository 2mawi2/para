import { ParaFinishArgs } from "../types/command-args.js";
import { runParaCommand } from "../utils/command-executor.js";

/**
 * Handle para_finish command
 */
export async function handleParaFinish(args: ParaFinishArgs, paraBinary: string): Promise<string> {
  const cmdArgs = ["finish"];
  
  cmdArgs.push(args.commit_message);
  
  if (args.session) {
    cmdArgs.push(args.session);
  }
  
  if (args.branch) {
    cmdArgs.push("--branch", args.branch);
  }
  
  return await runParaCommand(cmdArgs, paraBinary);
}

/**
 * MCP tool definition for para_finish
 */
export const paraFinishTool = {
  name: "para_finish",
  description: "Rarely used by orchestrator. Creates branch without merging. Agents use CLI 'para finish' command instead. Only use if you started a manual session with para_start and want to save work without merging.",
  inputSchema: {
    type: "object",
    properties: {
      commit_message: {
        type: "string",
        description: "Commit message describing the changes made"
      },
      session: {
        type: "string",
        description: "Session ID (optional, auto-detects if not provided)"
      },
      branch: {
        type: "string",
        description: "Custom branch name instead of default para/session-name. If branch already exists, error with suggestion."
      }
    },
    required: ["commit_message"]
  }
} as const;