import { BaseCommandHandler } from "../base-handler.js";
import { ToolDefinition, ToolResult, ParaStatusShowArgs } from "../types.js";
import { runParaCommand } from "../para-utils.js";

export class ParaStatusHandler extends BaseCommandHandler {
  constructor(private paraBinary: string) {
    super();
  }

  getToolDefinition(): ToolDefinition {
    return {
      name: "para_status_show",
      description: "Monitor agent progress across para sessions. Get agent-reported status including current task, test results (whole codebase health), confidence levels, todo progress, and blocked status. Use this to coordinate parallel development and identify agents needing assistance.\n\nOUTPUT INCLUDES:\n- current_task: What the agent is currently working on\n- test_status: passed/failed/unknown (reflects ALL tests in codebase)\n- confidence: high/medium/low (agent's self-assessment)\n- is_blocked: Whether agent needs help\n- todo_percentage: Progress through tasks\n- last_update: When status was last reported\n\nORCHESTRATOR USAGE:\n- Monitor all agents: para_status_show()\n- Check specific agent: para_status_show(session: 'agent-name')\n- Get structured data: para_status_show(json: true)\n\nREAD-ONLY: This tool only reads status. Agents update their own status via CLI.",
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
  }

  validateArgs(args: Record<string, unknown>): void {
    this.validateArgTypes(args, {
      session: "string",
      json: "boolean"
    });
  }

  async execute(args: Record<string, unknown>): Promise<ToolResult> {
    this.validateArgs(args);
    const statusArgs = args as ParaStatusShowArgs;
    
    const cmdArgs = ["status", "show"];
    if (statusArgs.session) {
      cmdArgs.push(statusArgs.session);
    }
    if (statusArgs.json) {
      cmdArgs.push("--json");
    }
    
    const result = await runParaCommand(cmdArgs, this.paraBinary);
    
    return {
      content: [
        {
          type: "text",
          text: result
        }
      ]
    };
  }
}