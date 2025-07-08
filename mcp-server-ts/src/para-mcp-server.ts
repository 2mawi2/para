#!/usr/bin/env node
/**
 * Para MCP Server - TypeScript implementation using official SDK
 * Calls into the Rust para binary for actual functionality
 */

import { Server } from "@modelcontextprotocol/sdk/server/index.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import {
  CallToolRequestSchema,
  ErrorCode,
  ListToolsRequestSchema,
  ListResourcesRequestSchema,
  ReadResourceRequestSchema,
  McpError,
} from "@modelcontextprotocol/sdk/types.js";

import { findParaBinary } from "./binary-discovery";
import { PARA_TOOLS } from "./tool-definitions";
import { PARA_RESOURCES } from "./resource-definitions";
import { runParaCommand } from "./command-execution";
import {
  StartCommandBuilder,
  FinishCommandBuilder,
  ResumeCommandBuilder,
  ListCommandBuilder,
  RecoverCommandBuilder,
  CancelCommandBuilder,
  StatusCommandBuilder,
  ConfigCommandBuilder
} from "./command-builders/index";
import {
  ParaStartArgs,
  ParaFinishArgs,
  ParaResumeArgs,
  ParaListArgs,
  ParaRecoverArgs,
  ParaCancelArgs,
  ParaStatusShowArgs,
  ParaConfigSetArgs
} from "./types";

const PARA_BINARY = findParaBinary();
console.error(`Para MCP server using para binary: ${PARA_BINARY}`);

const server = new Server({
  name: "para-mcp-server",
  version: "1.1.2",
}, {
  capabilities: {
    tools: {},
    resources: {},
  }
});

server.setRequestHandler(ListToolsRequestSchema, async () => {
  return {
    tools: PARA_TOOLS
  };
});

server.setRequestHandler(CallToolRequestSchema, async (request) => {
  const { name, arguments: args } = request.params;

  try {
    let result: string;

    switch (name) {
      case "para_start":
        {
          const startArgs = args as ParaStartArgs;
          const cmdArgs = StartCommandBuilder.build(startArgs);
          result = await runParaCommand(PARA_BINARY, cmdArgs);
        }
        break;

      case "para_finish":
        {
          const finishArgs = args as unknown as ParaFinishArgs;
          const cmdArgs = FinishCommandBuilder.build(finishArgs);
          result = await runParaCommand(PARA_BINARY, cmdArgs);
        }
        break;

      case "para_resume":
        {
          const resumeArgs = args as ParaResumeArgs;
          const cmdArgs = ResumeCommandBuilder.build(resumeArgs);
          result = await runParaCommand(PARA_BINARY, cmdArgs);
        }
        break;

      case "para_list":
        {
          const listArgs = args as ParaListArgs;
          const cmdArgs = ListCommandBuilder.build(listArgs);
          result = await runParaCommand(PARA_BINARY, cmdArgs);
        }
        break;

      case "para_recover":
        {
          const recoverArgs = args as ParaRecoverArgs;
          const cmdArgs = RecoverCommandBuilder.build(recoverArgs);
          result = await runParaCommand(PARA_BINARY, cmdArgs);
        }
        break;

      case "para_config_show":
        {
          const cmdArgs = ConfigCommandBuilder.buildShow();
          result = await runParaCommand(PARA_BINARY, cmdArgs);
        }
        break;

      case "para_config_set":
        {
          const configSetArgs = args as unknown as ParaConfigSetArgs;
          const cmdArgs = ConfigCommandBuilder.buildSet(configSetArgs);
          result = await runParaCommand(PARA_BINARY, cmdArgs);
        }
        break;

      case "para_cancel":
        {
          const cancelArgs = args as ParaCancelArgs;
          const cmdArgs = CancelCommandBuilder.build(cancelArgs);
          result = await runParaCommand(PARA_BINARY, cmdArgs);
        }
        break;

      case "para_status_show":
        {
          const statusArgs = args as ParaStatusShowArgs;
          const cmdArgs = StatusCommandBuilder.build(statusArgs);
          result = await runParaCommand(PARA_BINARY, cmdArgs);
        }
        break;

      default:
        throw new McpError(ErrorCode.MethodNotFound, `Unknown tool: ${name}`);
    }

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
});

server.setRequestHandler(ListResourcesRequestSchema, async () => {
  return {
    resources: PARA_RESOURCES
  };
});

server.setRequestHandler(ReadResourceRequestSchema, async (request) => {
  const { uri } = request.params;

  try {
    let content: string;

    switch (uri) {
      case "para://current-session":
        content = await runParaCommand(PARA_BINARY, ["list", "--current"]);
        break;

      case "para://config":
        content = await runParaCommand(PARA_BINARY, ["config", "show"]);
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
  } catch (error: unknown) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    throw new McpError(ErrorCode.InternalError, `Resource read failed: ${errorMessage}`);
  }
});

async function main(): Promise<void> {
  const transport = new StdioServerTransport();
  await server.connect(transport);
  console.error("Para MCP server running via TypeScript");
}

main().catch(console.error);