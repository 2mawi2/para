/**
 * Para status show tool implementation
 */

import { ToolDefinition } from '../types/mcp-types.js';
import { ParaStatusShowArgs } from '../types/para-args.js';
import { ParaExecutor } from '../execution/para-executor.js';

export const statusShowToolDefinition: ToolDefinition = {
  name: "para_status_show",
  description: "Monitor agent progress across para sessions. Get agent-reported status including current task, test results (whole codebase health), todo progress, and blocked status. Use this to coordinate parallel development and identify agents needing assistance.\n\nOUTPUT INCLUDES:\n- current_task: What the agent is currently working on\n- test_status: passed/failed/unknown (reflects ALL tests in codebase)\n- is_blocked: Whether agent needs help\n- todo_percentage: Progress through tasks\n- last_update: When status was last reported\n\nORCHESTRATOR USAGE:\n- Monitor all agents: para_status_show()\n- Check specific agent: para_status_show(session: 'agent-name')\n- Get structured data: para_status_show(json: true)\n\nREAD-ONLY: This tool only reads status. Agents update their own status via CLI.",
  inputSchema: {
    type: "object",
    properties: {
      session: {
        type: "string",
        description: "Session name to get status for (optional, shows all sessions if not provided)"
      },
      json: {
        type: "boolean",
        description: "Return structured JSON data instead of human-readable format",
        default: false
      }
    },
    additionalProperties: false
  }
};

export async function executeStatusShowTool(args: ParaStatusShowArgs, executor: ParaExecutor): Promise<string> {
  const cmdArgs = ["status", "show"];
  
  if (args.session) {
    cmdArgs.push(args.session);
  }
  if (args.json) {
    cmdArgs.push("--json");
  }
  
  return executor.runCommand(cmdArgs);
}