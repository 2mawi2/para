/**
 * Tool Handlers Module
 * 
 * Handles execution of MCP tool calls by mapping them to para commands.
 * Extracted from the monolithic para-mcp-server.ts for better modularity.
 */

import { McpError, ErrorCode } from "@modelcontextprotocol/sdk/types.js";
import { ParaCommandExecutor } from "./command-executor.js";
import { ToolName, isValidToolName } from "./tools/index.js";

export class ToolHandlers {
  private readonly executor: ParaCommandExecutor;

  constructor(executor: ParaCommandExecutor) {
    this.executor = executor;
  }

  /**
   * Handle a tool call request
   * @param name Tool name
   * @param args Tool arguments
   * @returns Tool execution result
   */
  public async handleToolCall(name: string, args: any): Promise<{ content: Array<{ type: string; text: string }> }> {
    if (!isValidToolName(name)) {
      throw new McpError(ErrorCode.MethodNotFound, `Unknown tool: ${name}`);
    }

    try {
      const result = await this.executeToolCommand(name, args);
      return {
        content: [
          {
            type: "text",
            text: result
          }
        ]
      };
    } catch (error: any) {
      throw new McpError(ErrorCode.InternalError, `Tool execution failed: ${error.message}`);
    }
  }

  /**
   * Execute the para command for a specific tool
   * @param name Tool name
   * @param args Tool arguments
   * @returns Command output
   */
  private async executeToolCommand(name: ToolName, args: any): Promise<string> {
    switch (name) {
      case "para_start":
        return this.handleStartCommand(args);
      
      case "para_finish":
        return this.handleFinishCommand(args);
      
      case "para_dispatch":
        return this.handleDispatchCommand(args);
      
      case "para_list":
        return this.handleListCommand(args);
      
      case "para_recover":
        return this.handleRecoverCommand(args);
      
      case "para_resume":
        return this.handleResumeCommand(args);
      
      case "para_config_show":
        return this.handleConfigShowCommand(args);
      
      case "para_cancel":
        return this.handleCancelCommand(args);
      
      case "para_status_show":
        return this.handleStatusShowCommand(args);
      
      default:
        throw new Error(`Unhandled tool: ${name}`);
    }
  }

  private async handleStartCommand(args: any): Promise<string> {
    const cmdArgs = ["start"];
    if (args.session_name) {
      cmdArgs.push(args.session_name);
    }
    if (args.dangerously_skip_permissions) {
      cmdArgs.push("--dangerously-skip-permissions");
    }
    return this.executor.execute(cmdArgs);
  }

  private async handleFinishCommand(args: any): Promise<string> {
    const cmdArgs = ["finish"];
    cmdArgs.push(args.commit_message);
    if (args.session) {
      cmdArgs.push(args.session);
    }
    if (args.branch) {
      cmdArgs.push("--branch", args.branch);
    }
    return this.executor.execute(cmdArgs);
  }

  private async handleDispatchCommand(args: any): Promise<string> {
    const cmdArgs = ["dispatch"];
    cmdArgs.push(args.session_name);

    if (args.file) {
      cmdArgs.push("--file", args.file);
    } else if (args.task_description) {
      cmdArgs.push(args.task_description);
    }

    if (args.dangerously_skip_permissions) {
      cmdArgs.push("--dangerously-skip-permissions");
    }

    return this.executor.execute(cmdArgs);
  }

  private async handleListCommand(args: any): Promise<string> {
    const cmdArgs = ["list"];
    if (args.verbose) {
      cmdArgs.push("--verbose");
    }
    if (args.archived) {
      cmdArgs.push("--archived");
    }
    if (args.quiet) {
      cmdArgs.push("--quiet");
    }
    return this.executor.execute(cmdArgs);
  }

  private async handleRecoverCommand(args: any): Promise<string> {
    const cmdArgs = ["recover"];
    if (args.session_name) {
      cmdArgs.push(args.session_name);
    }
    return this.executor.execute(cmdArgs);
  }

  private async handleResumeCommand(args: any): Promise<string> {
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
    return this.executor.execute(cmdArgs);
  }

  private async handleConfigShowCommand(args: any): Promise<string> {
    return this.executor.execute(["config", "show"]);
  }

  private async handleCancelCommand(args: any): Promise<string> {
    const cmdArgs = ["cancel"];
    if (args.session_name) {
      cmdArgs.push(args.session_name);
    }
    if (args.force) {
      cmdArgs.push("--force");
    }
    return this.executor.execute(cmdArgs);
  }

  private async handleStatusShowCommand(args: any): Promise<string> {
    const cmdArgs = ["status", "show"];
    if (args.session) {
      cmdArgs.push(args.session);
    }
    if (args.json) {
      cmdArgs.push("--json");
    }
    return this.executor.execute(cmdArgs);
  }
}