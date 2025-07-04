/**
 * Para start tool implementation
 */

import { ToolDefinition } from '../types/mcp-types.js';
import { ParaStartArgs } from '../types/para-args.js';
import { ParaExecutor } from '../execution/para-executor.js';

export const startToolDefinition: ToolDefinition = {
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
      },
      container: {
        type: "boolean",
        description: "Run session in Docker container"
      },
      docker_args: {
        type: "array",
        items: { type: "string" },
        description: "Additional Docker arguments (e.g., ['-d'] for detached mode)"
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

export async function executeStartTool(args: ParaStartArgs, executor: ParaExecutor): Promise<string> {
  const cmdArgs = ["start"];
  
  if (args.session_name) {
    cmdArgs.push(args.session_name);
  }
  if (args.dangerously_skip_permissions) {
    cmdArgs.push("--dangerously-skip-permissions");
  }
  if (args.container) {
    cmdArgs.push("--container");
  }
  if (args.docker_args && args.docker_args.length > 0) {
    cmdArgs.push("--docker-args", ...args.docker_args);
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