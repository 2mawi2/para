#!/usr/bin/env node
/**
 * Para MCP Server - Modular TypeScript implementation using official SDK
 * Calls into the Rust para binary for actual functionality
 */

import { Server } from "@modelcontextprotocol/sdk/server/index.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import {
  CallToolRequestSchema,
  ListToolsRequestSchema,
  ListResourcesRequestSchema,
  ReadResourceRequestSchema,
} from "@modelcontextprotocol/sdk/types.js";

import { ToolRegistry } from './registry/tool-registry.js';
import { ResourceRegistry } from './registry/resource-registry.js';

const server = new Server({
  name: "para-mcp-server",
  version: "1.1.2",
}, {
  capabilities: {
    tools: {},
    resources: {},
  }
});

const toolRegistry = new ToolRegistry();
const resourceRegistry = new ResourceRegistry();

server.setRequestHandler(ListToolsRequestSchema, async () => {
  return {
    tools: toolRegistry.getToolDefinitions()
  };
});

server.setRequestHandler(CallToolRequestSchema, async (request) => {
  const { name, arguments: args } = request.params;
  return toolRegistry.executeTool(name, args);
});

server.setRequestHandler(ListResourcesRequestSchema, async () => {
  return {
    resources: resourceRegistry.getResourceDefinitions()
  };
});

server.setRequestHandler(ReadResourceRequestSchema, async (request) => {
  const { uri } = request.params;
  return resourceRegistry.readResource(uri);
});

async function main(): Promise<void> {
  const transport = new StdioServerTransport();
  await server.connect(transport);
  console.error("Para MCP server running via TypeScript");
}

main().catch(console.error);