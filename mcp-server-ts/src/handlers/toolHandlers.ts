#!/usr/bin/env node
/**
 * MCP Tool Handlers
 * Handles execution of MCP tool calls
 */

import { McpError, ErrorCode } from "@modelcontextprotocol/sdk/types.js";
import { ParaService } from "../services/paraService.js";

/**
 * Tool Handlers class that processes MCP tool calls
 */
export class ToolHandlers {
  private paraService: ParaService;

  constructor() {
    this.paraService = new ParaService();
  }

  /**
   * Handles a tool call request
   */
  async handleToolCall(name: string, args: any): Promise<{ content: Array<{ type: string; text: string }> }> {
    try {
      let result: string;

      switch (name) {
        case "para_start":
          result = await this.paraService.executeStart(args);
          break;

        case "para_finish":
          result = await this.paraService.executeFinish(args);
          break;

        case "para_dispatch":
          result = await this.paraService.executeDispatch(args);
          break;

        case "para_list":
          result = await this.paraService.executeList(args);
          break;

        case "para_recover":
          result = await this.paraService.executeRecover(args);
          break;

        case "para_resume":
          result = await this.paraService.executeResume(args);
          break;

        case "para_config_show":
          result = await this.paraService.executeConfigShow();
          break;

        case "para_cancel":
          result = await this.paraService.executeCancel(args);
          break;

        case "para_status_show":
          result = await this.paraService.executeStatusShow(args);
          break;

        default:
          throw new McpError(ErrorCode.MethodNotFound, `Unknown tool: ${name}`);
      }

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
}