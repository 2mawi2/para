import { BaseCommandHandler } from "../base-handler.js";
import { ToolDefinition, ToolResult, ParaResumeArgs } from "../types.js";
import { runParaCommand } from "../para-utils.js";

export class ParaResumeHandler extends BaseCommandHandler {
  constructor(private paraBinary: string) {
    super();
  }

  getToolDefinition(): ToolDefinition {
    return {
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
    };
  }

  validateArgs(args: Record<string, unknown>): void {
    this.validateArgTypes(args, {
      session: "string",
      prompt: "string",
      file: "string"
    });
  }

  async execute(args: Record<string, unknown>): Promise<ToolResult> {
    this.validateArgs(args);
    const resumeArgs = args as ParaResumeArgs;
    
    const cmdArgs = ["resume"];
    if (resumeArgs.session) {
      cmdArgs.push(resumeArgs.session);
    }
    if (resumeArgs.prompt) {
      cmdArgs.push("--prompt", resumeArgs.prompt);
    }
    if (resumeArgs.file) {
      cmdArgs.push("--file", resumeArgs.file);
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