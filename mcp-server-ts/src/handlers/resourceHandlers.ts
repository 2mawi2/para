#!/usr/bin/env node
/**
 * MCP Resource Handlers
 * Handles reading of MCP resources
 */

import { McpError, ErrorCode } from "@modelcontextprotocol/sdk/types.js";
import { ParaService } from "../services/paraService.js";

/**
 * Resource Handlers class that processes MCP resource read requests
 */
export class ResourceHandlers {
  private paraService: ParaService;

  constructor() {
    this.paraService = new ParaService();
  }

  /**
   * Handles a resource read request
   */
  async handleResourceRead(uri: string): Promise<{ contents: Array<{ uri: string; mimeType: string; text: string }> }> {
    try {
      let content: string;

      switch (uri) {
        case "para://current-session":
          content = await this.paraService.getCurrentSession();
          break;

        case "para://config":
          content = await this.paraService.executeConfigShow();
          break;

        default:
          throw new McpError(ErrorCode.InvalidRequest, `Unknown resource: ${uri}`);
      }

      return {
        contents: [
          {
            uri,
            mimeType: "application/json",
            text: content
          }
        ]
      };
    } catch (error: any) {
      throw new McpError(ErrorCode.InternalError, `Resource read failed: ${error.message}`);
    }
  }
}