#!/usr/bin/env node
/**
 * Para MCP Server - Modular TypeScript implementation
 * Refactored to use separate modules for better maintainability and testability
 */

import { ParaMcpServer } from "./server-coordinator.js";

// Start the server
async function main() {
  try {
    const server = new ParaMcpServer();
    await server.start();
  } catch (error) {
    console.error("Failed to start Para MCP server:", error);
    process.exit(1);
  }
}

main().catch(console.error);