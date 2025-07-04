/**
 * Tool registry for dynamic tool loading and management
 */

import { ToolDefinition, ToolResponse } from '../types/mcp-types.js';
import { ParaExecutor } from '../execution/para-executor.js';
import { McpError, ErrorCode } from "@modelcontextprotocol/sdk/types.js";

// Import all tool definitions and executors
import { startToolDefinition, executeStartTool } from '../tools/start-tool.js';
import { finishToolDefinition, executeFinishTool } from '../tools/finish-tool.js';
import { dispatchToolDefinition, executeDispatchTool } from '../tools/dispatch-tool.js';
import { listToolDefinition, executeListTool } from '../tools/list-tool.js';
import { recoverToolDefinition, executeRecoverTool } from '../tools/recover-tool.js';
import { resumeToolDefinition, executeResumeTool } from '../tools/resume-tool.js';
import { cancelToolDefinition, executeCancelTool } from '../tools/cancel-tool.js';
import { configShowToolDefinition, executeConfigShowTool } from '../tools/config-show-tool.js';
import { configSetToolDefinition, executeConfigSetTool } from '../tools/config-set-tool.js';
import { statusShowToolDefinition, executeStatusShowTool } from '../tools/status-show-tool.js';

// Note: Individual tools import their specific arg types internally

type ToolExecutor = (_args: any, _executor: ParaExecutor) => Promise<string>;

interface RegistryEntry {
  definition: ToolDefinition;
  executor: ToolExecutor;
}

export class ToolRegistry {
  private tools: Map<string, RegistryEntry> = new Map();
  private paraExecutor: ParaExecutor;

  constructor() {
    this.paraExecutor = new ParaExecutor();
    this.registerAllTools();
  }

  private registerAllTools(): void {
    // Register all para tools
    this.registerTool(startToolDefinition, executeStartTool);
    this.registerTool(finishToolDefinition, executeFinishTool);
    this.registerTool(dispatchToolDefinition, executeDispatchTool);
    this.registerTool(listToolDefinition, executeListTool);
    this.registerTool(recoverToolDefinition, executeRecoverTool);
    this.registerTool(resumeToolDefinition, executeResumeTool);
    this.registerTool(cancelToolDefinition, executeCancelTool);
    this.registerTool(configShowToolDefinition, (_args) => executeConfigShowTool(this.paraExecutor));
    this.registerTool(configSetToolDefinition, executeConfigSetTool);
    this.registerTool(statusShowToolDefinition, executeStatusShowTool);
  }

  private registerTool(definition: ToolDefinition, executor: ToolExecutor): void {
    this.tools.set(definition.name, { definition, executor });
  }

  getToolDefinitions(): ToolDefinition[] {
    return Array.from(this.tools.values()).map(entry => entry.definition);
  }

  async executeTool(name: string, args: any): Promise<ToolResponse> {
    const entry = this.tools.get(name);
    if (!entry) {
      throw new McpError(ErrorCode.MethodNotFound, `Unknown tool: ${name}`);
    }

    try {
      const result = await entry.executor(args, this.paraExecutor);
      return {
        content: [
          {
            type: "text",
            text: result
          }
        ]
      };
    } catch (error: unknown) {
      const errorMessage = error instanceof Error ? error.message : String(error);
      throw new McpError(ErrorCode.InternalError, `Tool execution failed: ${errorMessage}`);
    }
  }

  isValidTool(name: string): boolean {
    return this.tools.has(name);
  }
}