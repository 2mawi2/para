import { ParaResumeArgs } from "../types/command-args.js";
import { runParaCommand } from "../utils/command-executor.js";

/**
 * Handle para_resume command
 */
export async function handleParaResume(args: ParaResumeArgs, paraBinary: string): Promise<string> {
  const cmdArgs = ["resume"];
  
  if (args.session) {
    cmdArgs.push(args.session);
  }
  
  if (args.prompt) {
    cmdArgs.push("--prompt", args.prompt);
  }
  
  if (args.file) {
    cmdArgs.push("--file", args.file);
  }
  
  return await runParaCommand(cmdArgs, paraBinary);
}

/**
 * MCP tool definition for para_resume
 */
export const paraResumeTool = {
  name: "para_resume",
  description: "Resume an existing active session with optional additional context or instructions. Opens the session's worktree in your IDE.",
  inputSchema: {
    type: "object",
    properties: {
      session: {
        type: "string",
        description: "Session ID to resume (optional, shows list if not provided)"
      },
      prompt: {
        type: "string",
        description: "Additional prompt or instructions for the resumed session"
      },
      file: {
        type: "string",
        description: "Read additional instructions from specified file"
      }
    },
    required: []
  }
} as const;