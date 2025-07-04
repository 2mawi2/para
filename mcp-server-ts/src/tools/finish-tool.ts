/**
 * Para finish tool implementation
 */

import { ToolDefinition } from '../types/mcp-types.js';
import { ParaFinishArgs } from '../types/para-args.js';
import { ParaExecutor } from '../execution/para-executor.js';

export const finishToolDefinition: ToolDefinition = {
  name: "para_finish",
  description: "Complete session and create feature branch for review. Creates commit and branch from session work. Agents typically use CLI 'para finish' command instead.",
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
};

export async function executeFinishTool(args: ParaFinishArgs, executor: ParaExecutor): Promise<string> {
  const cmdArgs = ["finish"];
  
  cmdArgs.push(args.commit_message);
  if (args.session) {
    cmdArgs.push(args.session);
  }
  if (args.branch) {
    cmdArgs.push("--branch", args.branch);
  }
  
  return executor.runCommand(cmdArgs);
}