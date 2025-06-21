/**
 * Tests for tool discovery and registration functionality
 */

import { jest } from '@jest/globals';
import { ToolRegistry } from '../tools/registry.js';
import { McpError, ErrorCode } from '@modelcontextprotocol/sdk/types.js';

describe('Tool Registry', () => {
  let toolRegistry: ToolRegistry;

  beforeEach(() => {
    toolRegistry = new ToolRegistry();
  });

  describe('Tool Discovery', () => {
    test('should register all para tools', () => {
      const expectedTools = [
        'para_start',
        'para_finish', 
        'para_dispatch',
        'para_list',
        'para_recover',
        'para_config_show',
        'para_cancel',
        'para_status_show'
      ];
      
      const toolNames = toolRegistry.getToolNames();
      expect(toolNames).toEqual(expect.arrayContaining(expectedTools));
      expect(toolNames.length).toBe(8);
    });

    test('should validate tool schemas', () => {
      const tools = toolRegistry.getAllToolDefinitions();
      
      // Each tool should have proper structure
      tools.forEach(tool => {
        expect(tool).toHaveProperty('name');
        expect(tool).toHaveProperty('description');
        expect(tool).toHaveProperty('inputSchema');
        expect(tool.inputSchema).toHaveProperty('type');
        expect(tool.inputSchema).toHaveProperty('properties');
        expect(tool.inputSchema).toHaveProperty('required');
        expect(typeof tool.name).toBe('string');
        expect(typeof tool.description).toBe('string');
      });
    });

    test('should have proper descriptions for all tools', () => {
      const tools = toolRegistry.getAllToolDefinitions();
      
      tools.forEach(tool => {
        expect(tool.description.length).toBeGreaterThan(10); // Meaningful descriptions
        expect(tool.name.startsWith('para_')).toBe(true); // All tools start with para_
      });
    });

    test('should get individual tool definitions', () => {
      const dispatchTool = toolRegistry.getToolDefinition('para_dispatch');
      expect(dispatchTool).toBeDefined();
      expect(dispatchTool?.name).toBe('para_dispatch');
      expect(dispatchTool?.inputSchema.required).toContain('session_name');
    });

    test('should return undefined for unknown tools', () => {
      const unknownTool = toolRegistry.getToolDefinition('para_unknown');
      expect(unknownTool).toBeUndefined();
    });
  });

  describe('Tool Validation', () => {
    test('should validate required parameters for para_dispatch', () => {
      const validArgs = { session_name: 'test-session' };
      
      expect(() => {
        toolRegistry.validateToolCall('para_dispatch', validArgs);
      }).not.toThrow();
    });

    test('should reject missing required parameters', () => {
      const invalidArgs = {}; // Missing required session_name
      
      expect(() => {
        toolRegistry.validateToolCall('para_dispatch', invalidArgs);
      }).toThrow(McpError);
      
      expect(() => {
        toolRegistry.validateToolCall('para_dispatch', invalidArgs);
      }).toThrow('Missing required parameter: session_name');
    });

    test('should validate para_finish required parameters', () => {
      const validArgs = { commit_message: 'Test commit' };
      expect(() => {
        toolRegistry.validateToolCall('para_finish', validArgs);
      }).not.toThrow();
      
      const invalidArgs = {}; // Missing commit_message
      expect(() => {
        toolRegistry.validateToolCall('para_finish', invalidArgs);
      }).toThrow('Missing required parameter: commit_message');
    });

    test('should validate optional parameters', () => {
      const callWithOptional = { 
        session_name: 'test',
        dangerously_skip_permissions: true
      };
      
      expect(() => {
        toolRegistry.validateToolCall('para_start', callWithOptional);
      }).not.toThrow();
    });

    test('should handle tools with no required parameters', () => {
      expect(() => {
        toolRegistry.validateToolCall('para_config_show', {});
      }).not.toThrow();
      
      expect(() => {
        toolRegistry.validateToolCall('para_list', {});
      }).not.toThrow();
    });
  });

  describe('Tool Existence Checks', () => {
    test('should correctly identify existing tools', () => {
      expect(toolRegistry.hasToolName('para_start')).toBe(true);
      expect(toolRegistry.hasToolName('para_dispatch')).toBe(true);
      expect(toolRegistry.hasToolName('para_status_show')).toBe(true);
    });

    test('should correctly identify non-existing tools', () => {
      expect(toolRegistry.hasToolName('para_unknown')).toBe(false);
      expect(toolRegistry.hasToolName('invalid_tool')).toBe(false);
      expect(toolRegistry.hasToolName('')).toBe(false);
    });

    test('should handle unknown tool names in validation', () => {
      expect(() => {
        toolRegistry.validateToolCall('para_unknown', {});
      }).toThrow(McpError);
      
      expect(() => {
        toolRegistry.validateToolCall('para_unknown', {});
      }).toThrow('Unknown tool: para_unknown');
    });
  });

  describe('Tool Schema Details', () => {
    test('should have correct schema for para_dispatch', () => {
      const tool = toolRegistry.getToolDefinition('para_dispatch');
      expect(tool?.inputSchema.required).toEqual(['session_name']);
      expect(tool?.inputSchema.properties).toHaveProperty('session_name');
      expect(tool?.inputSchema.properties).toHaveProperty('task_description');
      expect(tool?.inputSchema.properties).toHaveProperty('file');
      expect(tool?.inputSchema.properties).toHaveProperty('dangerously_skip_permissions');
    });

    test('should have correct schema for para_list', () => {
      const tool = toolRegistry.getToolDefinition('para_list');
      expect(tool?.inputSchema.required).toEqual([]);
      expect(tool?.inputSchema.properties).toHaveProperty('verbose');
      expect(tool?.inputSchema.properties).toHaveProperty('archived');
      expect(tool?.inputSchema.properties).toHaveProperty('quiet');
      expect(tool?.inputSchema.additionalProperties).toBe(false);
    });
  });
});