import { Server } from "@modelcontextprotocol/sdk/server/index.js";

// Mock the MCP SDK components
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

// Mock all the extracted modules
jest.mock('./binary-discovery', () => ({
  findParaBinary: jest.fn().mockReturnValue('/mock/para'),
}));

jest.mock('./command-execution', () => ({
  runParaCommand: jest.fn().mockResolvedValue('mock result'),
}));

jest.mock('./tool-definitions', () => ({
  PARA_TOOLS: [
    {
      name: 'para_start',
      description: 'Start new para session',
      inputSchema: { type: 'object', properties: {}, required: [] }
    }
  ],
}));

jest.mock('./resource-definitions', () => ({
  PARA_RESOURCES: [
    {
      uri: 'para://test',
      name: 'Test Resource',
      description: 'Test resource',
      mimeType: 'application/json'
    }
  ],
}));

jest.mock('./command-builders/index', () => ({
  StartCommandBuilder: {
    build: jest.fn().mockReturnValue(['start', 'test']),
  },
  FinishCommandBuilder: {
    build: jest.fn().mockReturnValue(['finish', 'test']),
  },
  ResumeCommandBuilder: {
    build: jest.fn().mockReturnValue(['resume', 'test']),
  },
  ListCommandBuilder: {
    build: jest.fn().mockReturnValue(['list']),
  },
  RecoverCommandBuilder: {
    build: jest.fn().mockReturnValue(['recover']),
  },
  CancelCommandBuilder: {
    build: jest.fn().mockReturnValue(['cancel']),
  },
  StatusCommandBuilder: {
    build: jest.fn().mockReturnValue(['status', 'show']),
  },
  ConfigCommandBuilder: {
    buildShow: jest.fn().mockReturnValue(['config', 'show']),
    buildSet: jest.fn().mockReturnValue(['config', 'set', 'path', 'value']),
  },
}));

describe('Para MCP Server Integration', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  test('should initialize server with correct configuration', () => {
    const MockServer = Server as jest.MockedClass<typeof Server>;
    
    // Import the server module to trigger initialization
    require('./para-mcp-server');
    
    expect(MockServer).toHaveBeenCalledWith(
      {
        name: "para-mcp-server",
        version: "1.1.2",
      },
      {
        capabilities: {
          tools: {},
          resources: {},
        }
      }
    );
  });

  test('should have modular structure', () => {
    // Test that the modules are properly extracted and importable
    const binaryDiscovery = require('./binary-discovery');
    const toolDefinitions = require('./tool-definitions');
    const resourceDefinitions = require('./resource-definitions');
    const commandExecution = require('./command-execution');
    const commandBuilders = require('./command-builders/index');
    
    expect(binaryDiscovery.findParaBinary).toBeDefined();
    expect(toolDefinitions.PARA_TOOLS).toBeDefined();
    expect(resourceDefinitions.PARA_RESOURCES).toBeDefined();
    expect(commandExecution.runParaCommand).toBeDefined();
    expect(commandBuilders.StartCommandBuilder).toBeDefined();
  });
});