/**
 * Tests for para binary discovery and interaction functionality
 */

import { jest } from '@jest/globals';
import { exec, execSync } from 'child_process';
import { ParaBinaryInterface } from '../para/binary-interface.js';
import { McpError, ErrorCode } from '@modelcontextprotocol/sdk/types.js';

// Mock child_process module
jest.mock('child_process', () => ({
  exec: jest.fn(),
  execSync: jest.fn(),
}));

const mockExec = exec as jest.MockedFunction<typeof exec>;
const mockExecSync = execSync as jest.MockedFunction<typeof execSync>;

describe('Para Binary Interface', () => {
  let binaryInterface: ParaBinaryInterface;

  beforeEach(() => {
    jest.clearAllMocks();
    // Clear environment variables
    delete process.env.HOME;
    process.argv = ['node', '/usr/local/bin/para-mcp-server'];
    
    // Mock successful binary discovery by default
    mockExecSync.mockImplementation(() => Buffer.from('success'));
    
    // Suppress console.error during tests
    jest.spyOn(console, 'error').mockImplementation(() => {});
  });

  afterEach(() => {
    jest.restoreAllMocks();
  });

  describe('constructor and binary discovery', () => {
    test('should find homebrew para for homebrew MCP server', () => {
      process.argv[1] = '/opt/homebrew/bin/para-mcp-server';
      mockExecSync
        .mockImplementationOnce(() => { throw new Error('not found'); }) // First location fails
        .mockImplementationOnce(() => Buffer.from('success')); // Second location succeeds
      
      binaryInterface = new ParaBinaryInterface();
      expect(binaryInterface.getBinaryPath()).toBe('/usr/local/bin/para');
    });

    test('should find development build when available', () => {
      process.cwd = jest.fn().mockReturnValue('/home/user/para');
      mockExecSync.mockImplementation((cmd) => {
        if (cmd === 'test -x /home/user/para/target/release/para') {
          return Buffer.from('success');
        }
        throw new Error('not found');
      });
      
      binaryInterface = new ParaBinaryInterface();
      expect(binaryInterface.getBinaryPath()).toBe('/home/user/para/target/release/para');
    });

    test('should fallback to system PATH when no specific location found', () => {
      mockExecSync.mockImplementation(() => {
        throw new Error('not found');
      });
      
      binaryInterface = new ParaBinaryInterface();
      expect(binaryInterface.getBinaryPath()).toBe('para');
    });

    test('should handle missing HOME environment variable', () => {
      delete process.env.HOME;
      mockExecSync.mockImplementation(() => {
        throw new Error('not found');
      });
      
      binaryInterface = new ParaBinaryInterface();
      expect(binaryInterface.getBinaryPath()).toBe('para');
    });
  });

  describe('validateBinaryExists', () => {
    beforeEach(() => {
      binaryInterface = new ParaBinaryInterface();
    });

    test('should return true when binary exists', () => {
      mockExecSync.mockReturnValue(Buffer.from('success'));
      expect(binaryInterface.validateBinaryExists()).toBe(true);
    });

    test('should return false when binary does not exist', () => {
      mockExecSync.mockImplementation(() => { throw new Error('not found'); });
      expect(binaryInterface.validateBinaryExists()).toBe(false);
    });
  });

  describe('executeCommand', () => {
    beforeEach(() => {
      binaryInterface = new ParaBinaryInterface();
    });

    test('should properly quote arguments with spaces', async () => {
      mockExec.mockImplementation((command, options, callback) => {
        expect(command).toContain('"argument with spaces"');
        if (typeof callback === 'function') {
          callback(null, 'success', '');
        }
        return {} as any;
      });

      await binaryInterface.executeCommand('test', ['argument with spaces']);
    });

    test('should set non-interactive environment variables', async () => {
      mockExec.mockImplementation((command, options, callback) => {
        expect(options?.env?.PARA_NON_INTERACTIVE).toBe('1');
        expect(options?.env?.CI).toBe('1');
        if (typeof callback === 'function') {
          callback(null, 'success', '');
        }
        return {} as any;
      });

      await binaryInterface.executeCommand('test', []);
    });

    test('should handle command timeout', async () => {
      jest.useFakeTimers();
      
      mockExec.mockImplementation((command, options, callback) => {
        // Don't call callback, simulate hanging process
        const mockChild = {
          kill: jest.fn()
        };
        return mockChild as any;
      });

      const promise = binaryInterface.executeCommand('test', []);
      
      // Fast-forward time to trigger timeout
      jest.advanceTimersByTime(30000);
      
      await expect(promise).rejects.toThrow(McpError);
      await expect(promise).rejects.toThrow('timed out');
      
      jest.useRealTimers();
    });

    test('should handle command execution errors', async () => {
      mockExec.mockImplementation((command, options, callback) => {
        if (typeof callback === 'function') {
          callback(new Error('Command failed'), '', 'Error output');
        }
        return {} as any;
      });

      await expect(binaryInterface.executeCommand('test', [])).rejects.toThrow(McpError);
      await expect(binaryInterface.executeCommand('test', [])).rejects.toThrow('Para command failed');
    });

    test('should ignore warning messages in stderr', async () => {
      mockExec.mockImplementation((command, options, callback) => {
        if (typeof callback === 'function') {
          callback(null, 'success', 'warning: this is just a warning');
        }
        return {} as any;
      });

      const result = await binaryInterface.executeCommand('test', []);
      expect(result.stdout).toBe('success');
      expect(result.stderr).toBe('warning: this is just a warning');
    });

    test('should return both stdout and stderr', async () => {
      mockExec.mockImplementation((command, options, callback) => {
        if (typeof callback === 'function') {
          callback(null, 'output content', 'error content');
        }
        return {} as any;
      });

      const result = await binaryInterface.executeCommand('test', []);
      expect(result.stdout).toBe('output content');
      expect(result.stderr).toBe('error content');
    });
  });
});