/**
 * Integration tests for the complete MCP server functionality
 * These tests verify end-to-end workflows and component integration
 */

import { exec, execSync } from 'child_process';
import { jest } from '@jest/globals';

// Mock child_process module
jest.mock('child_process', () => ({
  exec: jest.fn(),
  execSync: jest.fn()
}));

const mockExec = exec as jest.MockedFunction<any>;
const mockExecSync = execSync as jest.MockedFunction<any>;

// Mock MCP SDK
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

jest.mock('@modelcontextprotocol/sdk/server/stdio.js', () => ({
  StdioServerTransport: jest.fn().mockImplementation(() => ({
    // Mock transport
  }))
}));

jest.mock('@modelcontextprotocol/sdk/types.js', () => ({
  McpError: MockMcpError,
  ErrorCode: MockErrorCode,
  CallToolRequestSchema: 'CallToolRequestSchema',
  ListToolsRequestSchema: 'ListToolsRequestSchema',
  ListResourcesRequestSchema: 'ListResourcesRequestSchema',
  ReadResourceRequestSchema: 'ReadResourceRequestSchema'
}));

// Integration test simulation of the complete server
class IntegrationTestServer {
  private paraBinary: string;
  private handlers: Map<string, Function> = new Map();

  constructor() {
    this.paraBinary = this.findParaBinary();
    this.setupHandlers();
  }

  private findParaBinary(): string {
    // Simplified binary discovery for testing
    const locations = [
      "/opt/homebrew/bin/para",
      "/usr/local/bin/para", 
      "para"
    ];

    for (const location of locations) {
      try {
        execSync(`test -x ${location}`, { stdio: 'ignore' });
        return location;
      } catch {
        continue;
      }
    }
    return "para";
  }

  private async runParaCommand(args: string[]): Promise<string> {
    return new Promise((resolve, reject) => {
      const quotedArgs = args.map(arg => {
        if (arg.includes(' ') && !arg.startsWith('"') && !arg.startsWith("'")) {
          return `"${arg.replace(/"/g, '\\"')}"`;
        }
        return arg;
      });

      const command = `${this.paraBinary} ${quotedArgs.join(' ')}`;
      
      const env = {
        ...process.env,
        PARA_NON_INTERACTIVE: '1',
        CI: '1'
      };

      const child = exec(command, { env }, (error, stdout, stderr) => {
        clearTimeout(timeout);
        
        if (error) {
          reject(new MockMcpError(MockErrorCode.InternalError, `Para command failed: ${error.message}`));
          return;
        }
        
        if (stderr && !stderr.includes('warning')) {
          console.error(`Para command warning: ${stderr}`);
        }
        
        resolve(stdout.trim());
      });

      const timeout = setTimeout(() => {
        child.kill();
        reject(new MockMcpError(MockErrorCode.InternalError, `Command timed out after 30 seconds: ${args.join(' ')}`));
      }, 30000);
    });
  }

  private setupHandlers() {
    // List tools handler
    this.handlers.set('ListToolsRequestSchema', async () => {
      return {
        tools: [
          {
            name: "para_start",
            description: "Start manual development session",
            inputSchema: {
              type: "object",
              properties: {
                session_name: { type: "string" }
              }
            }
          },
          {
            name: "para_dispatch", 
            description: "Dispatch AI agents",
            inputSchema: {
              type: "object",
              properties: {
                session_name: { type: "string" },
                task_description: { type: "string" }
              },
              required: ["session_name"]
            }
          }
        ]
      };
    });

    // Call tool handler
    this.handlers.set('CallToolRequestSchema', async (request: any) => {
      const { name, arguments: args } = request.params;

      let result: string;
      switch (name) {
        case "para_start":
          const startArgs = ["start"];
          if (args.session_name) startArgs.push(args.session_name);
          result = await this.runParaCommand(startArgs);
          break;

        case "para_dispatch":
          const dispatchArgs = ["dispatch", args.session_name];
          if (args.task_description) dispatchArgs.push(args.task_description);
          result = await this.runParaCommand(dispatchArgs);
          break;

        default:
          throw new MockMcpError(MockErrorCode.MethodNotFound, `Unknown tool: ${name}`);
      }

      return {
        content: [{
          type: "text",
          text: result
        }]
      };
    });

    // List resources handler
    this.handlers.set('ListResourcesRequestSchema', async () => {
      return {
        resources: [
          {
            uri: "para://current-session",
            name: "Current Session",
            mimeType: "application/json"
          }
        ]
      };
    });

    // Read resource handler
    this.handlers.set('ReadResourceRequestSchema', async (request: any) => {
      const { uri } = request.params;
      
      if (uri === "para://current-session") {
        const content = await this.runParaCommand(["list", "--current"]);
        return {
          contents: [{
            uri,
            mimeType: "application/json",
            text: content
          }]
        };
      }
      
      throw new MockMcpError(MockErrorCode.InvalidRequest, `Unknown resource: ${uri}`);
    });
  }

  async handleRequest(schema: string, request?: any) {
    const handler = this.handlers.get(schema);
    if (!handler) {
      throw new Error(`No handler for ${schema}`);
    }
    return handler(request);
  }
}

describe('Integration Tests', () => {
  let server: IntegrationTestServer;
  let consoleSpy: jest.SpyInstance;

  beforeEach(() => {
    jest.clearAllMocks();
    consoleSpy = jest.spyOn(console, 'error').mockImplementation();
    server = new IntegrationTestServer();
  });

  afterEach(() => {
    consoleSpy.mockRestore();
  });

  describe('Complete Workflow: Start Session', () => {
    test('should complete start session workflow from tool list to execution', async () => {
      // Step 1: List available tools
      const toolList = await server.handleRequest('ListToolsRequestSchema');
      expect(toolList.tools).toHaveLength(2);
      
      const startTool = toolList.tools.find(t => t.name === 'para_start');
      expect(startTool).toBeDefined();

      // Step 2: Execute start command
      const mockChild = { kill: jest.fn() };
      mockExec.mockImplementation((command, options, callback) => {
        expect(command).toBe('para start "test-session"');
        expect(options.env.PARA_NON_INTERACTIVE).toBe('1');
        if (callback) callback(null, 'Session started: test-session', '');
        return mockChild as any;
      });

      const startRequest = {
        params: {
          name: 'para_start',
          arguments: { session_name: 'test-session' }
        }
      };

      const result = await server.handleRequest('CallToolRequestSchema', startRequest);
      expect(result.content[0].text).toBe('Session started: test-session');
    });

    test('should handle start session with binary discovery', async () => {
      // Mock binary discovery
      mockExecSync
        .mockImplementationOnce(() => { throw new Error('not found'); }) // First location fails
        .mockImplementationOnce(() => ''); // Second location succeeds

      server = new IntegrationTestServer(); // Recreate to trigger binary discovery

      const mockChild = { kill: jest.fn() };
      mockExec.mockImplementation((command, options, callback) => {
        expect(command).toBe('/usr/local/bin/para start');
        if (callback) callback(null, 'Session started with discovered binary', '');
        return mockChild as any;
      });

      const startRequest = {
        params: {
          name: 'para_start',
          arguments: {}
        }
      };

      const result = await server.handleRequest('CallToolRequestSchema', startRequest);
      expect(result.content[0].text).toBe('Session started with discovered binary');
    });
  });

  describe('Complete Workflow: Dispatch Agent', () => {
    test('should complete dispatch workflow with task description', async () => {
      const mockChild = { kill: jest.fn() };
      mockExec.mockImplementation((command, options, callback) => {
        expect(command).toBe('para dispatch "auth-agent" "Implement user authentication"');
        expect(options.env.CI).toBe('1');
        if (callback) callback(null, 'Agent dispatched successfully', '');
        return mockChild as any;
      });

      const dispatchRequest = {
        params: {
          name: 'para_dispatch',
          arguments: { 
            session_name: 'auth-agent',
            task_description: 'Implement user authentication'
          }
        }
      };

      const result = await server.handleRequest('CallToolRequestSchema', dispatchRequest);
      expect(result.content[0].text).toBe('Agent dispatched successfully');
    });

    test('should handle dispatch with complex arguments and quoting', async () => {
      const mockChild = { kill: jest.fn() };
      mockExec.mockImplementation((command, options, callback) => {
        expect(command).toBe('para dispatch "complex agent" "Task with \\"quotes\\" and spaces"');
        if (callback) callback(null, 'Complex dispatch completed', '');
        return mockChild as any;
      });

      const dispatchRequest = {
        params: {
          name: 'para_dispatch',
          arguments: { 
            session_name: 'complex agent',
            task_description: 'Task with "quotes" and spaces'
          }
        }
      };

      const result = await server.handleRequest('CallToolRequestSchema', dispatchRequest);
      expect(result.content[0].text).toBe('Complex dispatch completed');
    });
  });

  describe('Complete Workflow: Resource Access', () => {
    test('should complete resource workflow from list to read', async () => {
      // Step 1: List available resources
      const resourceList = await server.handleRequest('ListResourcesRequestSchema');
      expect(resourceList.resources).toHaveLength(1);
      expect(resourceList.resources[0].uri).toBe('para://current-session');

      // Step 2: Read the resource
      const mockChild = { kill: jest.fn() };
      mockExec.mockImplementation((command, options, callback) => {
        expect(command).toBe('para list --current');
        if (callback) callback(null, '{"session": "current-session-info"}', '');
        return mockChild as any;
      });

      const readRequest = {
        params: {
          uri: 'para://current-session'
        }
      };

      const result = await server.handleRequest('ReadResourceRequestSchema', readRequest);
      expect(result.contents[0].text).toBe('{"session": "current-session-info"}');
      expect(result.contents[0].uri).toBe('para://current-session');
    });
  });

  describe('Error Scenarios', () => {
    test('should handle para command failures gracefully', async () => {
      const mockChild = { kill: jest.fn() };
      const commandError = new Error('Para binary not found');
      
      mockExec.mockImplementation((command, options, callback) => {
        if (callback) callback(commandError, '', '');
        return mockChild as any;
      });

      const startRequest = {
        params: {
          name: 'para_start',
          arguments: { session_name: 'test-session' }
        }
      };

      await expect(server.handleRequest('CallToolRequestSchema', startRequest))
        .rejects.toEqual(new MockMcpError(MockErrorCode.InternalError, 'Para command failed: Para binary not found'));
    });

    test('should handle timeout scenarios', async () => {
      jest.useFakeTimers();
      
      const mockChild = { kill: jest.fn() };
      
      mockExec.mockImplementation((command, options, callback) => {
        // Don't call callback to simulate hanging command
        return mockChild as any;
      });

      const startRequest = {
        params: {
          name: 'para_start',
          arguments: { session_name: 'hanging-session' }
        }
      };

      const promise = server.handleRequest('CallToolRequestSchema', startRequest);
      
      // Fast-forward time by 30 seconds
      jest.advanceTimersByTime(30000);
      
      await expect(promise).rejects.toEqual(
        new MockMcpError(MockErrorCode.InternalError, 'Command timed out after 30 seconds: start,hanging-session')
      );
      
      expect(mockChild.kill).toHaveBeenCalledTimes(1);
      
      jest.useRealTimers();
    });

    test('should handle unknown tool requests', async () => {
      const unknownRequest = {
        params: {
          name: 'unknown_tool',
          arguments: {}
        }
      };

      await expect(server.handleRequest('CallToolRequestSchema', unknownRequest))
        .rejects.toEqual(new MockMcpError(MockErrorCode.MethodNotFound, 'Unknown tool: unknown_tool'));
    });

    test('should handle unknown resource requests', async () => {
      const unknownRequest = {
        params: {
          uri: 'para://unknown-resource'
        }
      };

      await expect(server.handleRequest('ReadResourceRequestSchema', unknownRequest))
        .rejects.toEqual(new MockMcpError(MockErrorCode.InvalidRequest, 'Unknown resource: para://unknown-resource'));
    });
  });

  describe('Environment and Configuration', () => {
    test('should set proper environment variables for all commands', async () => {
      const mockChild = { kill: jest.fn() };
      
      mockExec.mockImplementation((command, options, callback) => {
        expect(options.env).toMatchObject({
          PARA_NON_INTERACTIVE: '1',
          CI: '1'
        });
        // Should preserve existing environment
        expect(options.env).toMatchObject(process.env);
        if (callback) callback(null, 'Command executed', '');
        return mockChild as any;
      });

      // Test with start command
      await server.handleRequest('CallToolRequestSchema', {
        params: { name: 'para_start', arguments: {} }
      });

      // Test with dispatch command
      await server.handleRequest('CallToolRequestSchema', {
        params: { name: 'para_dispatch', arguments: { session_name: 'test' } }
      });

      // Test with resource read
      await server.handleRequest('ReadResourceRequestSchema', {
        params: { uri: 'para://current-session' }
      });

      expect(mockExec).toHaveBeenCalledTimes(3);
    });

    test('should handle stderr warnings appropriately', async () => {
      const mockChild = { kill: jest.fn() };
      
      mockExec.mockImplementation((command, options, callback) => {
        if (callback) callback(null, 'Command output', 'warning: deprecated feature used');
        return mockChild as any;
      });

      const result = await server.handleRequest('CallToolRequestSchema', {
        params: { name: 'para_start', arguments: {} }
      });

      expect(result.content[0].text).toBe('Command output');
      expect(consoleSpy).not.toHaveBeenCalled(); // Warning should be ignored
    });

    test('should log non-warning stderr messages', async () => {
      const mockChild = { kill: jest.fn() };
      
      mockExec.mockImplementation((command, options, callback) => {
        if (callback) callback(null, 'Command output', 'error: something went wrong');
        return mockChild as any;
      });

      const result = await server.handleRequest('CallToolRequestSchema', {
        params: { name: 'para_start', arguments: {} }
      });

      expect(result.content[0].text).toBe('Command output');
      expect(consoleSpy).toHaveBeenCalledWith('Para command warning: error: something went wrong');
    });
  });

  describe('Binary Discovery Integration', () => {
    test('should discover binary from homebrew when MCP path indicates homebrew', async () => {
      const originalArgv = process.argv[1];
      process.argv[1] = '/opt/homebrew/lib/node_modules/para-mcp-server/build/para-mcp-server.js';
      
      mockExecSync.mockImplementationOnce(() => ''); // Homebrew location found
      
      server = new IntegrationTestServer(); // Recreate to trigger discovery
      
      const mockChild = { kill: jest.fn() };
      mockExec.mockImplementation((command, options, callback) => {
        expect(command).toBe('/opt/homebrew/bin/para start');
        if (callback) callback(null, 'Homebrew binary used', '');
        return mockChild as any;
      });

      const result = await server.handleRequest('CallToolRequestSchema', {
        params: { name: 'para_start', arguments: {} }
      });

      expect(result.content[0].text).toBe('Homebrew binary used');
      
      process.argv[1] = originalArgv;
    });
  });
});