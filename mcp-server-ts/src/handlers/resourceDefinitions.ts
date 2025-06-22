#!/usr/bin/env node
/**
 * MCP Resource Definitions
 * Defines all the MCP resources available
 */

/**
 * Returns the list of available MCP resources
 */
export function getResourceDefinitions() {
  return [
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