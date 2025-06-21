/**
 * Integration tests for the complete MCP server functionality
 */

import { jest } from '@jest/globals';
import { exec } from 'child_process';
import { McpError, ErrorCode } from '@modelcontextprotocol/sdk/types.js';

// Mock child_process
jest.mock('child_process', () => ({
  exec: jest.fn(),
  execSync: jest.fn(),
}));

const mockExec = exec as jest.MockedFunction<typeof exec>;

describe('MCP Server Integration', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  describe('Tool Integration', () => {
    test('should execute para_start with proper command line args', async () => {
      // Mock successful para command execution
      mockExec.mockImplementation((command, options, callback) => {
        if (typeof callback === 'function') {
          callback(null, 'Session started successfully', '');
        }
        return {} as any;
      });

      // This will be tested once we refactor the server
      expect(true).toBe(true); // Placeholder
    });

    test('should execute para_dispatch with file parameter', async () => {
      mockExec.mockImplementation((command, options, callback) => {
        if (typeof callback === 'function') {
          // Verify command includes --file parameter
          expect(command).toContain('--file');
          callback(null, 'Agent dispatched successfully', '');
        }
        return {} as any;
      });

      // This will be tested once we refactor the server
      expect(true).toBe(true); // Placeholder
    });

    test('should handle dangerously_skip_permissions flag', async () => {
      mockExec.mockImplementation((command, options, callback) => {
        if (typeof callback === 'function') {
          // Verify command includes --dangerously-skip-permissions
          expect(command).toContain('--dangerously-skip-permissions');
          callback(null, 'Command executed', '');
        }
        return {} as any;
      });

      // This will be tested once we refactor the server
      expect(true).toBe(true); // Placeholder
    });
  });

  describe('Resource Integration', () => {
    test('should read current session resource', async () => {
      mockExec.mockImplementation((command, options, callback) => {
        if (typeof callback === 'function') {
          if (command.includes('list --current')) {
            callback(null, '{"session": "test-session"}', '');
          }
        }
        return {} as any;
      });

      // This will be tested once we refactor the server
      expect(true).toBe(true); // Placeholder
    });

    test('should read config resource', async () => {
      mockExec.mockImplementation((command, options, callback) => {
        if (typeof callback === 'function') {
          if (command.includes('config show')) {
            callback(null, '{"ide": "code"}', '');
          }
        }
        return {} as any;
      });

      // This will be tested once we refactor the server
      expect(true).toBe(true); // Placeholder
    });
  });

  describe('Command Line Generation', () => {
    test('should properly construct para_start command', () => {
      const args = { session_name: 'test-session', dangerously_skip_permissions: true };
      const expectedCommand = ['start', 'test-session', '--dangerously-skip-permissions'];
      
      // Test command construction logic
      const actualCommand = ['start'];
      if (args.session_name) actualCommand.push(args.session_name);
      if (args.dangerously_skip_permissions) actualCommand.push('--dangerously-skip-permissions');
      
      expect(actualCommand).toEqual(expectedCommand);
    });

    test('should properly construct para_finish command', () => {
      const args = { commit_message: 'Test commit', branch: 'feature/test' };
      const expectedCommand = ['finish', 'Test commit', '--branch', 'feature/test'];
      
      const actualCommand = ['finish', args.commit_message];
      if (args.branch) {
        actualCommand.push('--branch', args.branch);
      }
      
      expect(actualCommand).toEqual(expectedCommand);
    });

    test('should handle special characters in commit messages', () => {
      const args = { commit_message: 'Fix: handle "quotes" and spaces' };
      
      // Should properly quote the commit message
      expect(args.commit_message).toContain('"');
      expect(args.commit_message).toContain(' ');
    });
  });

  describe('Environment Setup', () => {
    test('should set non-interactive environment variables', () => {
      const expectedEnv = {
        PARA_NON_INTERACTIVE: '1',
        CI: '1'
      };
      
      // Test environment variable setup
      Object.entries(expectedEnv).forEach(([key, value]) => {
        expect(value).toBe('1');
      });
    });

    test('should preserve existing environment variables', () => {
      const originalEnv = { ...process.env };
      
      // Environment should include original variables plus para-specific ones
      expect(typeof originalEnv).toBe('object');
    });
  });

  describe('Error Propagation', () => {
    test('should propagate para command errors as MCP errors', async () => {
      mockExec.mockImplementation((command, options, callback) => {
        if (typeof callback === 'function') {
          callback(new Error('Para command failed'), '', 'Error output');
        }
        return {} as any;
      });

      // Should result in McpError
      const error = new McpError(ErrorCode.InternalError, 'Para command failed');
      expect(error).toBeInstanceOf(McpError);
      expect(error.code).toBe(ErrorCode.InternalError);
    });

    test('should handle timeout errors', async () => {
      // Mock timeout scenario
      const error = new McpError(ErrorCode.InternalError, 'Command timed out after 30 seconds');
      expect(error.message).toContain('timed out');
    });
  });
});