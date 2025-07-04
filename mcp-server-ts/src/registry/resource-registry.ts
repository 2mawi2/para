/**
 * Resource registry for MCP resources
 */

import { ResourceDefinition, ResourceResponse } from '../types/mcp-types.js';
import { ParaExecutor } from '../execution/para-executor.js';
import { McpError, ErrorCode } from "@modelcontextprotocol/sdk/types.js";

interface ResourceHandler {
  definition: ResourceDefinition;
  handler: (_executor: ParaExecutor) => Promise<string>;
}

export class ResourceRegistry {
  private resources: Map<string, ResourceHandler> = new Map();
  private paraExecutor: ParaExecutor;

  constructor() {
    this.paraExecutor = new ParaExecutor();
    this.registerAllResources();
  }

  private registerAllResources(): void {
    this.registerResource(
      {
        uri: "para://current-session",
        name: "Current Session",
        description: "Information about the current para session",
        mimeType: "application/json"
      },
      async (executor) => executor.runCommand(["list", "--current"])
    );

    this.registerResource(
      {
        uri: "para://config",
        name: "Para Configuration",
        description: "Current para configuration",
        mimeType: "application/json"
      },
      async (executor) => executor.runCommand(["config", "show"])
    );
  }

  private registerResource(definition: ResourceDefinition, handler: (_executor: ParaExecutor) => Promise<string>): void {
    this.resources.set(definition.uri, { definition, handler });
  }

  getResourceDefinitions(): ResourceDefinition[] {
    return Array.from(this.resources.values()).map(entry => entry.definition);
  }

  async readResource(uri: string): Promise<ResourceResponse> {
    const entry = this.resources.get(uri);
    if (!entry) {
      throw new McpError(ErrorCode.InvalidRequest, `Unknown resource: ${uri}`);
    }

    try {
      const content = await entry.handler(this.paraExecutor);
      return {
        contents: [
          {
            uri,
            mimeType: entry.definition.mimeType,
            text: content
          }
        ]
      };
    } catch (error: unknown) {
      const errorMessage = error instanceof Error ? error.message : String(error);
      throw new McpError(ErrorCode.InternalError, `Resource read failed: ${errorMessage}`);
    }
  }

  isValidResource(uri: string): boolean {
    return this.resources.has(uri);
  }
}