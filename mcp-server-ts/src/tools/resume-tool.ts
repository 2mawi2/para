/**
 * Para resume tool implementation
 */

import { ToolDefinition } from '../types/mcp-types.js';
import { ParaResumeArgs } from '../types/para-args.js';
import { ParaExecutor } from '../execution/para-executor.js';

export const resumeToolDefinition: ToolDefinition = {
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
      },
      dangerously_skip_permissions: {
        type: "boolean",
        description: "Skip IDE permission warnings (dangerous)"
      },
      sandbox: {
        type: "boolean",
        description: "Enable sandboxing for Claude CLI (overrides config)"
      },
      no_sandbox: {
        type: "boolean",
        description: "Disable sandboxing for Claude CLI (overrides config)"
      },
      sandbox_profile: {
        type: "string",
        description: "Sandbox profile to use: permissive (default) or restrictive"
      }
    },
    required: []
  }
};

export async function executeResumeTool(args: ParaResumeArgs, executor: ParaExecutor): Promise<string> {
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
  if (args.dangerously_skip_permissions) {
    cmdArgs.push("--dangerously-skip-permissions");
  }
  if (args.sandbox) {
    cmdArgs.push("--sandbox");
  }
  if (args.no_sandbox) {
    cmdArgs.push("--no-sandbox");
  }
  if (args.sandbox_profile) {
    cmdArgs.push("--sandbox-profile", args.sandbox_profile);
  }
  
  return executor.runCommand(cmdArgs);
}