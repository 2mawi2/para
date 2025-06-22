#!/usr/bin/env node
/**
 * Para MCP Server - Entry point for refactored modular implementation
 * This file now simply imports and starts the modular server
 */

import { ParaMcpServer } from "./server.js";

// Start the server
async function main() {
  const server = new ParaMcpServer();
  await server.start();
}

main().catch(console.error);