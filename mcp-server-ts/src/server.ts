#!/usr/bin/env node
/**
 * Para MCP Server - Refactored modular implementation
 * Main server setup and message routing
 */

import { Server } from "@modelcontextprotocol/sdk/server/index.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import {
  CallToolRequestSchema,
  ListToolsRequestSchema,
  ListResourcesRequestSchema,
  ReadResourceRequestSchema,
} from "@modelcontextprotocol/sdk/types.js";

import { getToolDefinitions } from "./handlers/toolDefinitions.js";
import { getResourceDefinitions } from "./handlers/resourceDefinitions.js";
import { ToolHandlers } from "./handlers/toolHandlers.js";
import { ResourceHandlers } from "./handlers/resourceHandlers.js";

/**
 * Main MCP Server class
 */
export class ParaMcpServer {
  private server: Server;
  private toolHandlers: ToolHandlers;
  private resourceHandlers: ResourceHandlers;

  constructor() {
    this.server = new Server({
      name: "para-mcp-server",
      version: "1.1.2",
    }, {
      capabilities: {
        tools: {},
        resources: {},
      }
    });

    this.toolHandlers = new ToolHandlers();
    this.resourceHandlers = new ResourceHandlers();

    this.setupHandlers();
  }

  /**
   * Sets up all the MCP request handlers
   */
  private setupHandlers(): void {
    // List available tools
    this.server.setRequestHandler(ListToolsRequestSchema, async () => {
      return {
        tools: getToolDefinitions()
      };
    });

    // Handle tool calls
    this.server.setRequestHandler(CallToolRequestSchema, async (request) => {
      const { name, arguments: args } = request.params;
      return await this.toolHandlers.handleToolCall(name, args);
    });

    // List available resources
    this.server.setRequestHandler(ListResourcesRequestSchema, async () => {
      return {
        resources: getResourceDefinitions()
      };
    });

    // Read resources
    this.server.setRequestHandler(ReadResourceRequestSchema, async (request) => {
      const { uri } = request.params;
      return await this.resourceHandlers.handleResourceRead(uri);
    });
  }

  /**
   * Starts the MCP server
   */
  async start(): Promise<void> {
    const transport = new StdioServerTransport();
    await this.server.connect(transport);
    console.error("Para MCP server running via TypeScript");
  }
}

// Start the server if this file is run directly
if (import.meta.url === `file://${process.argv[1]}`) {
  const server = new ParaMcpServer();
  server.start().catch(console.error);
}