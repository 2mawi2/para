/**
 * Server Coordinator - Main MCP server that coordinates all modules
 */

import { Server } from "@modelcontextprotocol/sdk/server/index.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import {
  CallToolRequestSchema,
  ListToolsRequestSchema,
  ListResourcesRequestSchema,
  ReadResourceRequestSchema,
  McpError,
  ErrorCode,
} from "@modelcontextprotocol/sdk/types.js";

import { ParaBinaryInterface } from "./para/binary-interface.js";
import { ToolRegistry } from "./tools/registry.js";
import { SessionTools } from "./tools/session-tools.js";
import { StatusTools } from "./tools/status-tools.js";
import { ManagementTools } from "./tools/management-tools.js";

export class ParaMcpServer {
  private server: Server;
  private binaryInterface: ParaBinaryInterface;
  private toolRegistry: ToolRegistry;
  private sessionTools: SessionTools;
  private statusTools: StatusTools;
  private managementTools: ManagementTools;

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

    // Initialize all modules
    this.binaryInterface = new ParaBinaryInterface();
    this.toolRegistry = new ToolRegistry();
    this.sessionTools = new SessionTools(this.binaryInterface);
    this.statusTools = new StatusTools(this.binaryInterface);
    this.managementTools = new ManagementTools(this.binaryInterface);

    this.setupHandlers();
  }

  /**
   * Set up all MCP request handlers
   */
  private setupHandlers(): void {
    this.setupToolHandlers();
    this.setupResourceHandlers();
  }

  /**
   * Set up tool-related handlers
   */
  private setupToolHandlers(): void {
    // List available tools
    this.server.setRequestHandler(ListToolsRequestSchema, async () => {
      return {
        tools: this.toolRegistry.getAllToolDefinitions()
      };
    });

    // Handle tool calls
    this.server.setRequestHandler(CallToolRequestSchema, async (request) => {
      const { name, arguments: args } = request.params;

      try {
        // Validate the tool call
        this.toolRegistry.validateToolCall(name, args);

        // Execute the appropriate tool
        const result = await this.executeToolCall(name, args);

        return {
          content: [
            {
              type: "text",
              text: result.stdout
            }
          ]
        };
      } catch (error: any) {
        if (error instanceof McpError) {
          throw error;
        }
        throw new McpError(ErrorCode.InternalError, `Tool execution failed: ${error.message}`);
      }
    });
  }

  /**
   * Execute a tool call by dispatching to the appropriate module
   */
  private async executeToolCall(name: string, args: any) {
    switch (name) {
      // Session Tools
      case "para_start":
        return await this.sessionTools.start(args);
      case "para_finish":
        return await this.sessionTools.finish(args);
      case "para_dispatch":
        return await this.sessionTools.dispatch(args);

      // Status Tools
      case "para_list":
        return await this.statusTools.list(args);
      case "para_status_show":
        return await this.statusTools.statusShow(args);

      // Management Tools
      case "para_recover":
        return await this.managementTools.recover(args);
      case "para_cancel":
        return await this.managementTools.cancel(args);
      case "para_config_show":
        return await this.managementTools.configShow();

      default:
        throw new McpError(ErrorCode.MethodNotFound, `Unknown tool: ${name}`);
    }
  }

  /**
   * Set up resource-related handlers
   */
  private setupResourceHandlers(): void {
    // List available resources
    this.server.setRequestHandler(ListResourcesRequestSchema, async () => {
      return {
        resources: [
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
        ]
      };
    });

    // Read resources
    this.server.setRequestHandler(ReadResourceRequestSchema, async (request) => {
      const { uri } = request.params;

      try {
        let content: string;

        switch (uri) {
          case "para://current-session":
            const currentSession = await this.statusTools.list({ quiet: true });
            content = currentSession.stdout;
            break;

          case "para://config":
            const config = await this.managementTools.configShow();
            content = config.stdout;
            break;

          default:
            throw new McpError(ErrorCode.InvalidRequest, `Unknown resource: ${uri}`);
        }

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
        if (error instanceof McpError) {
          throw error;
        }
        throw new McpError(ErrorCode.InternalError, `Resource read failed: ${error.message}`);
      }
    });
  }

  /**
   * Start the MCP server
   */
  async start(): Promise<void> {
    const transport = new StdioServerTransport();
    await this.server.connect(transport);
    console.error("Para MCP server running via TypeScript (modular)");
  }
}