/**
 * Resource Handlers Module
 * 
 * Handles MCP resource operations (list and read resources).
 * Extracted from the monolithic para-mcp-server.ts for better modularity.
 */

import { McpError, ErrorCode } from "@modelcontextprotocol/sdk/types.js";
import { ParaCommandExecutor } from "./command-executor.js";

export interface ResourceDefinition {
  uri: string;
  name: string;
  description: string;
  mimeType: string;
}

export interface ResourceContent {
  uri: string;
  mimeType: string;
  text: string;
}

export class ResourceHandlers {
  private readonly executor: ParaCommandExecutor;
  private readonly resources: ResourceDefinition[];

  constructor(executor: ParaCommandExecutor) {
    this.executor = executor;
    this.resources = [
      {
        uri: "para://current-session",
        name: "Current Session",
        description: "Information about the current para session",
        mimeType: "application/json"
      },
      {
        uri: "para://config",
        name: "Para Configuration",
        description: "Current para configuration",
        mimeType: "application/json"
      }
    ];
  }

  /**
   * List all available resources
   * @returns List of resource definitions
   */
  public async listResources(): Promise<{ resources: ResourceDefinition[] }> {
    return {
      resources: this.resources
    };
  }

  /**
   * Read a specific resource by URI
   * @param uri Resource URI
   * @returns Resource content
   */
  public async readResource(uri: string): Promise<{ contents: ResourceContent[] }> {
    try {
      const content = await this.getResourceContent(uri);
      
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

  /**
   * Get the content for a specific resource URI
   * @param uri Resource URI
   * @returns Resource content as string
   */
  private async getResourceContent(uri: string): Promise<string> {
    switch (uri) {
      case "para://current-session":
        return this.executor.execute(["list", "--current"]);
      
      case "para://config":
        return this.executor.execute(["config", "show"]);
      
      default:
        throw new McpError(ErrorCode.InvalidRequest, `Unknown resource: ${uri}`);
    }
  }

  /**
   * Check if a resource URI is valid
   * @param uri Resource URI
   * @returns True if the URI is valid
   */
  public isValidResource(uri: string): boolean {
    return this.resources.some(resource => resource.uri === uri);
  }
}