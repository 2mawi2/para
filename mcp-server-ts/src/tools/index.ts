/**
 * Tool Registry
 * 
 * Centralized registry for all MCP tools, organized by category.
 * Exports unified tool list and type definitions.
 */

import { sessionTools, SessionToolName } from "./session-tools.js";
import { dispatchTools, DispatchToolName } from "./dispatch-tools.js";
import { managementTools, ManagementToolName } from "./management-tools.js";

// Combined tool list
export const allTools = [
  ...sessionTools,
  ...dispatchTools,
  ...managementTools
];

// Union type of all tool names
export type ToolName = SessionToolName | DispatchToolName | ManagementToolName;

// Tool categories for organization
export const toolCategories = {
  session: sessionTools,
  dispatch: dispatchTools,
  management: managementTools
} as const;

// Helper function to get tool by name
export function getToolByName(name: string) {
  return allTools.find(tool => tool.name === name);
}

// Helper function to check if a tool name is valid
export function isValidToolName(name: string): name is ToolName {
  return allTools.some(tool => tool.name === name);
}