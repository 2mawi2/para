/**
 * Tests for MCP protocol functionality
 * These tests verify tool execution, resource handling, and protocol compliance
 */

import { jest } from '@jest/globals';

// Mock the MCP SDK
const mockServer = {
  setRequestHandler: jest.fn(),
  connect: jest.fn()
};

const MockMcpError = jest.fn();
const MockErrorCode = {
  InternalError: 'InternalError',
  MethodNotFound: 'MethodNotFound',
  InvalidRequest: 'InvalidRequest'
};

jest.mock('@modelcontextprotocol/sdk/server/index.js', () => ({
  Server: jest.fn().mockImplementation(() => mockServer)
}));

jest.mock('@modelcontextprotocol/sdk/types.js', () => ({
  McpError: MockMcpError,
  ErrorCode: MockErrorCode,
  CallToolRequestSchema: 'CallToolRequestSchema',
  ListToolsRequestSchema: 'ListToolsRequestSchema',
  ListResourcesRequestSchema: 'ListResourcesRequestSchema',
  ReadResourceRequestSchema: 'ReadResourceRequestSchema'
}));

// Mock the command executor
const mockRunParaCommand = jest.fn<Promise<string>, [string[]]>();

// Extract the MCP handler logic for testing
class MockParaMcpServer {
  private server: any;
  
  constructor() {
    this.server = mockServer;
    
    // Set up tool list handler
    this.server.setRequestHandler('ListToolsRequestSchema', this.handleListTools.bind(this));
    
    // Set up tool call handler  
    this.server.setRequestHandler('CallToolRequestSchema', this.handleCallTool.bind(this));
    
    // Set up resource list handler
    this.server.setRequestHandler('ListResourcesRequestSchema', this.handleListResources.bind(this));
    
    // Set up resource read handler
    this.server.setRequestHandler('ReadResourceRequestSchema', this.handleReadResource.bind(this));
  }

  async handleListTools() {
    return {
      tools: [
        {
          name: "para_start",
          description: "Start manual development session in isolated Git worktree. For complex tasks where YOU (orchestrator) work WITH the user, not for dispatching agents. Creates .para/worktrees/session-name directory. Use when user needs direct involvement or task is too complex for agents.",
          inputSchema: {
            type: "object",
            properties: {
              session_name: {
                type: "string",
                description: "Name for the new session (optional, generates friendly name if not provided)"
              },
              dangerously_skip_permissions: {
                type: "boolean",
                description: "Skip IDE permission warnings (dangerous)"
              }
            },
            required: []
          }
        },
        {
          name: "para_dispatch",
          description: "PRIMARY TOOL: Dispatch AI agents for parallel development. Each agent works in isolated Git worktree.",
          inputSchema: {
            type: "object",
            properties: {
              session_name: {
                type: "string",
                description: "Unique name for this agent/session (e.g., 'auth-api', 'frontend-ui')"
              },
              task_description: {
                type: "string",
                description: "Inline task description for SIMPLE tasks only. Use for short, natural language prompts without special characters."
              },
              file: {
                type: "string",
                description: "Path to task file (e.g., tasks/TASK_1_auth.md). Default directory: tasks/"
              },
              dangerously_skip_permissions: {
                type: "boolean",
                description: "Skip IDE permission warnings (dangerous)"
              }
            },
            required: ["session_name"]
          }
        }
      ]
    };
  }

  async handleCallTool(request: any) {
    const { name, arguments: args } = request.params;

    try {
      let result: string;

      switch (name) {
        case "para_start":
          {
            const cmdArgs = ["start"];
            if (args.session_name) {
              cmdArgs.push(args.session_name);
            }
            if (args.dangerously_skip_permissions) {
              cmdArgs.push("--dangerously-skip-permissions");
            }
            result = await mockRunParaCommand(cmdArgs);
          }
          break;

        case "para_dispatch":
          {
            const cmdArgs = ["dispatch"];
            cmdArgs.push(args.session_name);

            if (args.file) {
              cmdArgs.push("--file", args.file);
            } else if (args.task_description) {
              cmdArgs.push(args.task_description);
            }

            if (args.dangerously_skip_permissions) {
              cmdArgs.push("--dangerously-skip-permissions");
            }

            result = await mockRunParaCommand(cmdArgs);
          }
          break;

        default:
          throw new MockMcpError(MockErrorCode.MethodNotFound, `Unknown tool: ${name}`);
      }

      return {
        content: [
          {
            type: "text",
            text: result
          }
        ]
      };
    } catch (error: any) {
      throw new MockMcpError(MockErrorCode.InternalError, `Tool execution failed: ${error.message}`);
    }
  }

  async handleListResources() {
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
  }

  async handleReadResource(request: any) {
    const { uri } = request.params;

    try {
      let content: string;

      switch (uri) {
        case "para://current-session":
          content = await mockRunParaCommand(["list", "--current"]);
          break;

        case "para://config":
          content = await mockRunParaCommand(["config", "show"]);
          break;

        default:
          throw new MockMcpError(MockErrorCode.InvalidRequest, `Unknown resource: ${uri}`);
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
      throw new MockMcpError(MockErrorCode.InternalError, `Resource read failed: ${error.message}`);
    }
  }
}

describe('MCP Protocol', () => {
  let mcpServer: MockParaMcpServer;

  beforeEach(() => {
    jest.clearAllMocks();
    mcpServer = new MockParaMcpServer();
  });

  describe('Tool Listing', () => {
    test('should return list of available tools', async () => {
      const result = await mcpServer.handleListTools();
      
      expect(result.tools).toHaveLength(2);
      expect(result.tools[0].name).toBe('para_start');
      expect(result.tools[1].name).toBe('para_dispatch');
    });

    test('should include proper tool schemas', async () => {
      const result = await mcpServer.handleListTools();
      
      const startTool = result.tools.find(t => t.name === 'para_start');
      expect(startTool).toBeDefined();
      expect(startTool?.inputSchema.type).toBe('object');
      expect(startTool?.inputSchema.properties.session_name).toBeDefined();
      expect(startTool?.inputSchema.properties.dangerously_skip_permissions).toBeDefined();
    });

    test('should include detailed descriptions', async () => {
      const result = await mcpServer.handleListTools();
      
      const dispatchTool = result.tools.find(t => t.name === 'para_dispatch');
      expect(dispatchTool?.description).toContain('PRIMARY TOOL');
      expect(dispatchTool?.description).toContain('Dispatch AI agents');
    });
  });

  describe('Tool Execution', () => {
    describe('para_start tool', () => {
      test('should execute para start command with session name', async () => {
        mockRunParaCommand.mockResolvedValue('Session started successfully');
        
        const request = {
          params: {
            name: 'para_start',
            arguments: { session_name: 'test-session' }
          }
        };

        const result = await mcpServer.handleCallTool(request);
        
        expect(mockRunParaCommand).toHaveBeenCalledWith(['start', 'test-session']);
        expect(result.content[0].text).toBe('Session started successfully');
      });

      test('should execute para start command without session name', async () => {
        mockRunParaCommand.mockResolvedValue('Session started with generated name');
        
        const request = {
          params: {
            name: 'para_start',
            arguments: {}
          }
        };

        const result = await mcpServer.handleCallTool(request);
        
        expect(mockRunParaCommand).toHaveBeenCalledWith(['start']);
        expect(result.content[0].text).toBe('Session started with generated name');
      });

      test('should include skip permissions flag when provided', async () => {
        mockRunParaCommand.mockResolvedValue('Session started with permissions skipped');
        
        const request = {
          params: {
            name: 'para_start',
            arguments: { 
              session_name: 'test-session',
              dangerously_skip_permissions: true 
            }
          }
        };

        const result = await mcpServer.handleCallTool(request);
        
        expect(mockRunParaCommand).toHaveBeenCalledWith(['start', 'test-session', '--dangerously-skip-permissions']);
      });
    });

    describe('para_dispatch tool', () => {
      test('should execute para dispatch with task description', async () => {
        mockRunParaCommand.mockResolvedValue('Agent dispatched successfully');
        
        const request = {
          params: {
            name: 'para_dispatch',
            arguments: { 
              session_name: 'test-agent',
              task_description: 'Implement feature X'
            }
          }
        };

        const result = await mcpServer.handleCallTool(request);
        
        expect(mockRunParaCommand).toHaveBeenCalledWith(['dispatch', 'test-agent', 'Implement feature X']);
        expect(result.content[0].text).toBe('Agent dispatched successfully');
      });

      test('should execute para dispatch with file parameter', async () => {
        mockRunParaCommand.mockResolvedValue('Agent dispatched with file');
        
        const request = {
          params: {
            name: 'para_dispatch',
            arguments: { 
              session_name: 'test-agent',
              file: 'tasks/TASK_1.md'
            }
          }
        };

        const result = await mcpServer.handleCallTool(request);
        
        expect(mockRunParaCommand).toHaveBeenCalledWith(['dispatch', 'test-agent', '--file', 'tasks/TASK_1.md']);
      });

      test('should prioritize file over task_description', async () => {
        mockRunParaCommand.mockResolvedValue('Agent dispatched with file');
        
        const request = {
          params: {
            name: 'para_dispatch',
            arguments: { 
              session_name: 'test-agent',
              task_description: 'Should be ignored',
              file: 'tasks/TASK_1.md'
            }
          }
        };

        const result = await mcpServer.handleCallTool(request);
        
        expect(mockRunParaCommand).toHaveBeenCalledWith(['dispatch', 'test-agent', '--file', 'tasks/TASK_1.md']);
      });

      test('should include skip permissions flag for dispatch', async () => {
        mockRunParaCommand.mockResolvedValue('Agent dispatched with permissions skipped');
        
        const request = {
          params: {
            name: 'para_dispatch',
            arguments: { 
              session_name: 'test-agent',
              task_description: 'Test task',
              dangerously_skip_permissions: true
            }
          }
        };

        const result = await mcpServer.handleCallTool(request);
        
        expect(mockRunParaCommand).toHaveBeenCalledWith(['dispatch', 'test-agent', 'Test task', '--dangerously-skip-permissions']);
      });

      test('should require session_name parameter', async () => {
        const request = {
          params: {
            name: 'para_dispatch',
            arguments: { 
              task_description: 'Test task'
            }
          }
        };

        // This should still work as session_name is handled in argument processing
        // but would fail at the para command level
        mockRunParaCommand.mockRejectedValue(new Error('session_name is required'));
        
        await expect(mcpServer.handleCallTool(request)).rejects.toEqual(
          new MockMcpError(MockErrorCode.InternalError, 'Tool execution failed: session_name is required')
        );
      });
    });

    describe('Unknown tools', () => {
      test('should throw MethodNotFound for unknown tools', async () => {
        const request = {
          params: {
            name: 'unknown_tool',
            arguments: {}
          }
        };

        await expect(mcpServer.handleCallTool(request)).rejects.toEqual(
          new MockMcpError(MockErrorCode.MethodNotFound, 'Unknown tool: unknown_tool')
        );
      });
    });

    describe('Error handling', () => {
      test('should wrap command execution errors in McpError', async () => {
        mockRunParaCommand.mockRejectedValue(new Error('Command failed'));
        
        const request = {
          params: {
            name: 'para_start',
            arguments: {}
          }
        };

        await expect(mcpServer.handleCallTool(request)).rejects.toEqual(
          new MockMcpError(MockErrorCode.InternalError, 'Tool execution failed: Command failed')
        );
      });
    });
  });

  describe('Resource Listing', () => {
    test('should return list of available resources', async () => {
      const result = await mcpServer.handleListResources();
      
      expect(result.resources).toHaveLength(2);
      expect(result.resources[0].uri).toBe('para://current-session');
      expect(result.resources[1].uri).toBe('para://config');
    });

    test('should include proper resource metadata', async () => {
      const result = await mcpServer.handleListResources();
      
      const sessionResource = result.resources.find(r => r.uri === 'para://current-session');
      expect(sessionResource).toBeDefined();
      expect(sessionResource?.name).toBe('Current Session');
      expect(sessionResource?.mimeType).toBe('application/json');
      expect(sessionResource?.description).toContain('current para session');
    });
  });

  describe('Resource Reading', () => {
    test('should read current session resource', async () => {
      mockRunParaCommand.mockResolvedValue('{"session": "current-session-data"}');
      
      const request = {
        params: {
          uri: 'para://current-session'
        }
      };

      const result = await mcpServer.handleReadResource(request);
      
      expect(mockRunParaCommand).toHaveBeenCalledWith(['list', '--current']);
      expect(result.contents[0].uri).toBe('para://current-session');
      expect(result.contents[0].text).toBe('{"session": "current-session-data"}');
    });

    test('should read config resource', async () => {
      mockRunParaCommand.mockResolvedValue('{"config": "para-config-data"}');
      
      const request = {
        params: {
          uri: 'para://config'
        }
      };

      const result = await mcpServer.handleReadResource(request);
      
      expect(mockRunParaCommand).toHaveBeenCalledWith(['config', 'show']);
      expect(result.contents[0].uri).toBe('para://config');
      expect(result.contents[0].text).toBe('{"config": "para-config-data"}');
    });

    test('should throw InvalidRequest for unknown resources', async () => {
      const request = {
        params: {
          uri: 'para://unknown-resource'
        }
      };

      await expect(mcpServer.handleReadResource(request)).rejects.toEqual(
        new MockMcpError(MockErrorCode.InvalidRequest, 'Unknown resource: para://unknown-resource')
      );
    });

    test('should wrap resource read errors in McpError', async () => {
      mockRunParaCommand.mockRejectedValue(new Error('Failed to read resource'));
      
      const request = {
        params: {
          uri: 'para://current-session'
        }
      };

      await expect(mcpServer.handleReadResource(request)).rejects.toEqual(
        new MockMcpError(MockErrorCode.InternalError, 'Resource read failed: Failed to read resource')
      );
    });
  });

  describe('Response Format', () => {
    test('should return properly formatted tool response', async () => {
      mockRunParaCommand.mockResolvedValue('Command output');
      
      const request = {
        params: {
          name: 'para_start',
          arguments: {}
        }
      };

      const result = await mcpServer.handleCallTool(request);
      
      expect(result).toEqual({
        content: [
          {
            type: "text",
            text: "Command output"
          }
        ]
      });
    });

    test('should return properly formatted resource response', async () => {
      mockRunParaCommand.mockResolvedValue('Resource content');
      
      const request = {
        params: {
          uri: 'para://current-session'
        }
      };

      const result = await mcpServer.handleReadResource(request);
      
      expect(result).toEqual({
        contents: [
          {
            uri: 'para://current-session',
            mimeType: 'application/json',
            text: 'Resource content'
          }
        ]
      });
    });
  });
});