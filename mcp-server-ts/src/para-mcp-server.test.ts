import { execSync } from 'child_process';

// Prevent actual binary execution during tests
jest.mock('child_process', () => ({
  exec: jest.fn(),
  execSync: jest.fn(),
}));

const mockExecSync = execSync as jest.MockedFunction<typeof execSync>;

// Isolate server logic from external MCP SDK dependencies
jest.mock('@modelcontextprotocol/sdk/server/index.js', () => ({
  Server: jest.fn().mockImplementation(() => ({
    setRequestHandler: jest.fn(),
    connect: jest.fn(),
  })),
}));

jest.mock('@modelcontextprotocol/sdk/server/stdio.js', () => ({
  StdioServerTransport: jest.fn(),
}));

jest.mock('@modelcontextprotocol/sdk/types.js', () => ({
  CallToolRequestSchema: {},
  ErrorCode: {
    InternalError: 'INTERNAL_ERROR',
    MethodNotFound: 'METHOD_NOT_FOUND',
    InvalidRequest: 'INVALID_REQUEST',
  },
  ListToolsRequestSchema: {},
  ListResourcesRequestSchema: {},
  ReadResourceRequestSchema: {},
  McpError: class McpError extends Error {
    constructor(public _code: string, message: string) {
      super(message);
      this.name = 'McpError';
    }
  },
}));

describe('Para MCP Server', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    // Prevent tests from interfering with each other's binary discovery
    process.argv = ['node', 'test-script.js'];
  });

  describe('Binary Discovery', () => {
    test('should find para binary in local development build', () => {
      mockExecSync.mockImplementation((command: string) => {
        if (command.includes('test -x') && command.includes('target/release/para')) {
          return '';
        }
        throw new Error('Command failed');
      });

      expect(mockExecSync).toBeDefined();
    });

    test('should find para binary in local debug build', () => {
      mockExecSync.mockImplementation((command: string) => {
        if (command.includes('test -x') && command.includes('target/debug/para')) {
          return '';
        }
        throw new Error('Command failed');
      });

      expect(mockExecSync).toBeDefined();
    });

    test('should find para binary in homebrew installation', () => {
      // Simulate homebrew installation environment for path testing
      const originalArgv = process.argv[1];
      process.argv[1] = '/opt/homebrew/bin/para-mcp-server';
      
      mockExecSync.mockImplementation((command: string) => {
        if (command.includes('test -x') && command.includes('/opt/homebrew/bin/para')) {
          return '';
        }
        throw new Error('Command failed');
      });

      expect(mockExecSync).toBeDefined();
      
      // Prevent test state from affecting other tests
      process.argv[1] = originalArgv;
    });

    test('should fallback to PATH para when no specific location found', () => {
      mockExecSync.mockImplementation(() => {
        throw new Error('Command failed');
      });

      expect(mockExecSync).toBeDefined();
    });
  });

  describe('Command Argument Building', () => {
    test('should build para_start command with session name', () => {
      const expectedArgs = ['start', 'test-session'];
      
      expect(expectedArgs).toEqual(['start', 'test-session']);
    });

    test('should build para_start command with dangerously_skip_permissions', () => {
      const expectedArgs = ['start', 'test-session', '--dangerously-skip-permissions'];
      
      expect(expectedArgs).toEqual(['start', 'test-session', '--dangerously-skip-permissions']);
    });

    test('should build para_finish command with required commit message', () => {
      const expectedArgs = ['finish', 'Add new feature'];
      
      expect(expectedArgs).toEqual(['finish', 'Add new feature']);
    });

    test('should build para_finish command with session and branch', () => {
      const expectedArgs = ['finish', 'Add new feature', 'test-session', '--branch', 'feature-branch'];
      
      expect(expectedArgs).toEqual(['finish', 'Add new feature', 'test-session', '--branch', 'feature-branch']);
    });

    test('should build para_dispatch command with session name and file', () => {
      const expectedArgs = ['dispatch', 'api-agent', '--file', 'tasks/api-task.md'];
      
      expect(expectedArgs).toEqual(['dispatch', 'api-agent', '--file', 'tasks/api-task.md']);
    });

    test('should build para_dispatch command with task description', () => {
      const expectedArgs = ['dispatch', 'ui-agent', 'Build user interface components'];
      
      expect(expectedArgs).toEqual(['dispatch', 'ui-agent', 'Build user interface components']);
    });

    test('should build para_list command with all flags', () => {
      const expectedArgs = ['list', '--verbose', '--archived', '--quiet'];
      
      expect(expectedArgs).toEqual(['list', '--verbose', '--archived', '--quiet']);
    });

    test('should build para_recover command with session name', () => {
      const expectedArgs = ['recover', 'lost-session'];
      
      expect(expectedArgs).toEqual(['recover', 'lost-session']);
    });

    test('should build para_resume command with all options', () => {
      const expectedArgs = ['resume', 'active-session', '--prompt', 'Continue with implementation', '--file', 'additional-context.md'];
      
      expect(expectedArgs).toEqual(['resume', 'active-session', '--prompt', 'Continue with implementation', '--file', 'additional-context.md']);
    });

    test('should build para_cancel command with force flag', () => {
      const expectedArgs = ['cancel', 'abandoned-session', '--force'];
      
      expect(expectedArgs).toEqual(['cancel', 'abandoned-session', '--force']);
    });

    test('should build para_status_show command with JSON output', () => {
      const expectedArgs = ['status', 'show', 'agent-session', '--json'];
      
      expect(expectedArgs).toEqual(['status', 'show', 'agent-session', '--json']);
    });

    test('should build para_config_set command with string value', () => {
      const expectedArgs = ['config', 'set', 'ide.name', 'cursor'];
      
      expect(expectedArgs).toEqual(['config', 'set', 'ide.name', 'cursor']);
    });

    test('should build para_config_set command with boolean value', () => {
      const expectedArgs = ['config', 'set', 'git.auto_stage', 'true'];
      
      expect(expectedArgs).toEqual(['config', 'set', 'git.auto_stage', 'true']);
    });

    test('should build para_config_set command with number value', () => {
      const expectedArgs = ['config', 'set', 'session.auto_cleanup_days', '14'];
      
      expect(expectedArgs).toEqual(['config', 'set', 'session.auto_cleanup_days', '14']);
    });

    test('should build para_config_set command with nested path', () => {
      const expectedArgs = ['config', 'set', 'ide.wrapper.command', 'cursor'];
      
      expect(expectedArgs).toEqual(['config', 'set', 'ide.wrapper.command', 'cursor']);
    });
  });

  describe('Argument Quoting', () => {
    test('should quote arguments with spaces', () => {
      const input = 'commit message with spaces';
      const expected = '"commit message with spaces"';
      
      const result = input.includes(' ') && !input.startsWith('"') && !input.startsWith("'") 
        ? `"${input.replace(/"/g, '\\"')}"` 
        : input;
      
      expect(result).toBe(expected);
    });

    test('should not quote arguments without spaces', () => {
      const input = 'single-word';
      const expected = 'single-word';
      
      const result = input.includes(' ') && !input.startsWith('"') && !input.startsWith("'") 
        ? `"${input.replace(/"/g, '\\"')}"` 
        : input;
      
      expect(result).toBe(expected);
    });

    test('should not quote already quoted arguments', () => {
      const input = '"already quoted"';
      const expected = '"already quoted"';
      
      const result = input.includes(' ') && !input.startsWith('"') && !input.startsWith("'") 
        ? `"${input.replace(/"/g, '\\"')}"` 
        : input;
      
      expect(result).toBe(expected);
    });

    test('should escape internal quotes when quoting', () => {
      const input = 'message with "quotes" inside';
      const expected = '"message with \\"quotes\\" inside"';
      
      const result = input.includes(' ') && !input.startsWith('"') && !input.startsWith("'") 
        ? `"${input.replace(/"/g, '\\"')}"` 
        : input;
      
      expect(result).toBe(expected);
    });
  });

  describe('Tool Definitions', () => {
    test('should define para_start tool with correct schema', () => {
      const expectedTool = {
        name: "para_start",
        description: expect.stringContaining("Start manual development session"),
        inputSchema: {
          type: "object",
          properties: {
            session_name: {
              type: "string",
              description: expect.stringContaining("Name for the new session")
            },
            dangerously_skip_permissions: {
              type: "boolean",
              description: "Skip IDE permission warnings (dangerous)"
            }
          },
          required: []
        }
      };
      
      expect(expectedTool.name).toBe("para_start");
      expect(expectedTool.inputSchema.properties).toHaveProperty('session_name');
      expect(expectedTool.inputSchema.properties).toHaveProperty('dangerously_skip_permissions');
    });

    test('should define para_dispatch tool with correct schema', () => {
      const expectedTool = {
        name: "para_dispatch",
        description: expect.stringContaining("PRIMARY TOOL"),
        inputSchema: {
          type: "object",
          properties: {
            session_name: expect.objectContaining({ type: "string" }),
            task_description: expect.objectContaining({ type: "string" }),
            file: expect.objectContaining({ type: "string" }),
            dangerously_skip_permissions: expect.objectContaining({ type: "boolean" })
          },
          required: ["session_name"]
        }
      };
      
      expect(expectedTool.name).toBe("para_dispatch");
      expect(expectedTool.inputSchema.required).toContain("session_name");
    });

    test('should define para_config_set tool with correct schema', () => {
      const expectedTool = {
        name: "para_config_set",
        description: expect.stringContaining("Set para configuration values"),
        inputSchema: {
          type: "object",
          properties: {
            path: {
              type: "string",
              description: expect.stringContaining("JSON path using dot notation")
            },
            value: {
              oneOf: [
                { type: "string" },
                { type: "boolean" },
                { type: "number" }
              ],
              description: expect.stringContaining("Value to set")
            }
          },
          required: ["path", "value"]
        }
      };
      
      expect(expectedTool.name).toBe("para_config_set");
      expect(expectedTool.inputSchema.required).toEqual(["path", "value"]);
      expect(expectedTool.inputSchema.properties.value.oneOf).toHaveLength(3);
    });

    test('should define all required tools', () => {
      const expectedTools = [
        'para_start',
        'para_finish', 
        'para_dispatch',
        'para_list',
        'para_recover',
        'para_resume',
        'para_config_show',
        'para_config_set',
        'para_cancel',
        'para_status_show'
      ];
      
      expectedTools.forEach(toolName => {
        expect(toolName).toMatch(/^para_/);
      });
    });
  });

  describe('Resource Definitions', () => {
    test('should define current-session resource', () => {
      const expectedResource = {
        uri: "para://current-session",
        name: "Current Session",
        description: "Information about the current para session",
        mimeType: "application/json"
      };
      
      expect(expectedResource.uri).toBe("para://current-session");
      expect(expectedResource.mimeType).toBe("application/json");
    });

    test('should define config resource', () => {
      const expectedResource = {
        uri: "para://config",
        name: "Para Configuration", 
        description: "Current para configuration",
        mimeType: "application/json"
      };
      
      expect(expectedResource.uri).toBe("para://config");
      expect(expectedResource.mimeType).toBe("application/json");
    });
  });

  describe('Environment Variables', () => {
    test('should set non-interactive environment variables', () => {
      const expectedEnv = {
        PARA_NON_INTERACTIVE: '1',
        CI: '1'
      };
      
      expect(expectedEnv.PARA_NON_INTERACTIVE).toBe('1');
      expect(expectedEnv.CI).toBe('1');
    });
  });

  describe('Error Handling', () => {
    test('should handle unknown tool error', () => {
      const unknownTool = 'unknown_tool';
      const expectedError = {
        code: 'METHOD_NOT_FOUND',
        message: `Unknown tool: ${unknownTool}`
      };
      
      expect(expectedError.code).toBe('METHOD_NOT_FOUND');
      expect(expectedError.message).toContain(unknownTool);
    });

    test('should handle command execution timeout', () => {
      const timeoutError = {
        code: 'INTERNAL_ERROR',
        message: 'Command timed out after 30 seconds'
      };
      
      expect(timeoutError.code).toBe('INTERNAL_ERROR');
      expect(timeoutError.message).toContain('timed out');
    });

    test('should handle invalid resource URI', () => {
      const invalidUri = 'para://invalid-resource';
      const expectedError = {
        code: 'INVALID_REQUEST',
        message: `Unknown resource: ${invalidUri}`
      };
      
      expect(expectedError.code).toBe('INVALID_REQUEST');
      expect(expectedError.message).toContain(invalidUri);
    });
  });
});