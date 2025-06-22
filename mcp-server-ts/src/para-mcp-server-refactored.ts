#!/usr/bin/env node
/**
 * Para MCP Server - Refactored TypeScript implementation
 * 
 * This is the refactored version that uses modular components:
 * - ParaBinaryFinder: Discovers para binary location
 * - ParaCommandExecutor: Executes para commands with proper handling
 * - ToolHandlers: Handles MCP tool calls
 * - ResourceHandlers: Handles MCP resource operations
 * - Tool definitions organized by category
 * 
 * This replaces the monolithic para-mcp-server.ts with cleaner architecture.
 */

import { Server } from "@modelcontextprotocol/sdk/server/index.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import {
  CallToolRequestSchema,
  ListToolsRequestSchema,
  ListResourcesRequestSchema,
  ReadResourceRequestSchema,
} from "@modelcontextprotocol/sdk/types.js";

import { ParaBinaryFinder } from "./binary-finder.js";
import { ParaCommandExecutor } from "./command-executor.js";
import { ToolHandlers } from "./tool-handlers.js";
import { ResourceHandlers } from "./resource-handlers.js";
import { allTools } from "./tools/index.js";

class ParaMcpServer {
  private readonly server: Server;
  private readonly toolHandlers: ToolHandlers;
  private readonly resourceHandlers: ResourceHandlers;

  constructor() {
    // Initialize core components
    const paraBinary = ParaBinaryFinder.findBinary();
    console.error(`Para MCP server using para binary: ${paraBinary}`);
    
    const executor = new ParaCommandExecutor(paraBinary);
    this.toolHandlers = new ToolHandlers(executor);
    this.resourceHandlers = new ResourceHandlers(executor);

    // Initialize MCP server
    this.server = new Server({
      name: "para-mcp-server",
      version: "1.1.2",
    }, {
      capabilities: {
        tools: {},
        resources: {},
      }
    });

    this.setupRequestHandlers();
  }

  /**
   * Set up all MCP request handlers
   */
  private setupRequestHandlers(): void {
    // List available tools
    this.server.setRequestHandler(ListToolsRequestSchema, async () => {
      return { tools: allTools };
    });

    // Handle tool calls
    this.server.setRequestHandler(CallToolRequestSchema, async (request) => {
      const { name, arguments: args } = request.params;
      return this.toolHandlers.handleToolCall(name, args);
    });

    // List available resources
    this.server.setRequestHandler(ListResourcesRequestSchema, async () => {
      return this.resourceHandlers.listResources();
    });

    // Read resources
    this.server.setRequestHandler(ReadResourceRequestSchema, async (request) => {
      const { uri } = request.params;
      return this.resourceHandlers.readResource(uri);
    });
  }

  /**
   * Start the MCP server
   */
  public async start(): Promise<void> {
    const transport = new StdioServerTransport();
    await this.server.connect(transport);
    console.error("Para MCP server running via TypeScript (refactored)");
  }
}

// Start the server
async function main() {
  const mcpServer = new ParaMcpServer();
  await mcpServer.start();
}

main().catch(console.error);