/**
 * Tests for para command execution functionality
 * These tests verify command execution, timeout handling, and error management
 */

import { exec } from 'child_process';
import { jest } from '@jest/globals';

// Mock child_process module
jest.mock('child_process', () => ({
  exec: jest.fn(),
  execSync: jest.fn()
}));

const mockExec = exec as jest.MockedFunction<any>;

// Mock McpError
const MockMcpError = jest.fn();
jest.mock('@modelcontextprotocol/sdk/types.js', () => ({
  McpError: MockMcpError,
  ErrorCode: {
    InternalError: 'InternalError'
  }
}));

// Extract the command execution logic for testing
// This represents the current logic from para-mcp-server.ts lines 85-127
async function runParaCommand(args: string[], paraBinary: string = 'para'): Promise<string> {
  return new Promise((resolve, reject) => {
    // Properly quote arguments that contain spaces
    const quotedArgs = args.map(arg => {
      // If the argument contains spaces and isn't already quoted, wrap it in quotes
      if (arg.includes(' ') && !arg.startsWith('"') && !arg.startsWith("'")) {
        return `"${arg.replace(/"/g, '\\"')}"`;
      }
      return arg;
    });

    const command = `${paraBinary} ${quotedArgs.join(' ')}`;
    
    // Set environment to indicate non-interactive mode
    const env = {
      ...process.env,
      PARA_NON_INTERACTIVE: '1',
      CI: '1'  // Many CLIs respect this as well
    };

    const child = exec(command, { env }, (error, stdout, stderr) => {
      clearTimeout(timeout);
      
      if (error) {
        reject(new MockMcpError('InternalError', `Para command failed: ${error.message}`));
        return;
      }
      
      if (stderr && !stderr.includes('warning')) {
        console.error(`Para command warning: ${stderr}`);
      }
      
      resolve(stdout.trim());
    });

    // Set a 30-second timeout
    const timeout = setTimeout(() => {
      child.kill();
      reject(new MockMcpError('InternalError', `Command timed out after 30 seconds: ${args.join(' ')}`));
    }, 30000);
  });
}

describe('Command Executor', () => {
  let consoleSpy: jest.SpyInstance;

  beforeEach(() => {
    jest.clearAllMocks();
    consoleSpy = jest.spyOn(console, 'error').mockImplementation();
  });

  afterEach(() => {
    consoleSpy.mockRestore();
  });

  describe('Argument Quoting', () => {
    test('should quote arguments containing spaces', async () => {
      const mockChild = {
        kill: jest.fn()
      };
      
      mockExec.mockImplementation((command, options, callback) => {
        expect(command).toBe('para start "my session name"');
        if (callback) callback(null, 'success', '');
        return mockChild as any;
      });

      await runParaCommand(['start', 'my session name']);
      expect(mockExec).toHaveBeenCalledTimes(1);
    });

    test('should not double-quote already quoted arguments', async () => {
      const mockChild = {
        kill: jest.fn()
      };
      
      mockExec.mockImplementation((command, options, callback) => {
        expect(command).toBe('para start "my session"');
        if (callback) callback(null, 'success', '');
        return mockChild as any;
      });

      await runParaCommand(['start', '"my session"']);
      expect(mockExec).toHaveBeenCalledTimes(1);
    });

    test('should escape quotes within arguments', async () => {
      const mockChild = {
        kill: jest.fn()
      };
      
      mockExec.mockImplementation((command, options, callback) => {
        expect(command).toBe('para start "session with \\"quotes\\""');
        if (callback) callback(null, 'success', '');
        return mockChild as any;
      });

      await runParaCommand(['start', 'session with "quotes"']);
      expect(mockExec).toHaveBeenCalledTimes(1);
    });

    test('should handle arguments without spaces', async () => {
      const mockChild = {
        kill: jest.fn()
      };
      
      mockExec.mockImplementation((command, options, callback) => {
        expect(command).toBe('para start session');
        if (callback) callback(null, 'success', '');
        return mockChild as any;
      });

      await runParaCommand(['start', 'session']);
      expect(mockExec).toHaveBeenCalledTimes(1);
    });

    test('should handle single-quoted arguments', async () => {
      const mockChild = {
        kill: jest.fn()
      };
      
      mockExec.mockImplementation((command, options, callback) => {
        expect(command).toBe("para start 'my session'");
        if (callback) callback(null, 'success', '');
        return mockChild as any;
      });

      await runParaCommand(['start', "'my session'"]);
      expect(mockExec).toHaveBeenCalledTimes(1);
    });
  });

  describe('Environment Variables', () => {
    test('should set non-interactive environment variables', async () => {
      const mockChild = {
        kill: jest.fn()
      };
      
      mockExec.mockImplementation((command, options, callback) => {
        expect(options.env).toMatchObject({
          PARA_NON_INTERACTIVE: '1',
          CI: '1'
        });
        if (callback) callback(null, 'success', '');
        return mockChild as any;
      });

      await runParaCommand(['list']);
      expect(mockExec).toHaveBeenCalledTimes(1);
    });

    test('should preserve existing environment variables', async () => {
      const originalEnv = process.env;
      process.env = { ...originalEnv, CUSTOM_VAR: 'test_value' };
      
      const mockChild = {
        kill: jest.fn()
      };
      
      mockExec.mockImplementation((command, options, callback) => {
        expect(options.env).toMatchObject({
          CUSTOM_VAR: 'test_value',
          PARA_NON_INTERACTIVE: '1',
          CI: '1'
        });
        if (callback) callback(null, 'success', '');
        return mockChild as any;
      });

      await runParaCommand(['list']);
      
      process.env = originalEnv;
      expect(mockExec).toHaveBeenCalledTimes(1);
    });
  });

  describe('Success Cases', () => {
    test('should resolve with trimmed stdout on success', async () => {
      const mockChild = {
        kill: jest.fn()
      };
      
      mockExec.mockImplementation((command, options, callback) => {
        if (callback) callback(null, '  command output  \n', '');
        return mockChild as any;
      });

      const result = await runParaCommand(['list']);
      expect(result).toBe('command output');
    });

    test('should handle empty stdout', async () => {
      const mockChild = {
        kill: jest.fn()
      };
      
      mockExec.mockImplementation((command, options, callback) => {
        if (callback) callback(null, '', '');
        return mockChild as any;
      });

      const result = await runParaCommand(['list']);
      expect(result).toBe('');
    });

    test('should use custom para binary path', async () => {
      const mockChild = {
        kill: jest.fn()
      };
      
      mockExec.mockImplementation((command, options, callback) => {
        expect(command).toBe('/custom/path/para list');
        if (callback) callback(null, 'success', '');
        return mockChild as any;
      });

      await runParaCommand(['list'], '/custom/path/para');
      expect(mockExec).toHaveBeenCalledTimes(1);
    });
  });

  describe('Warning Handling', () => {
    test('should log stderr that does not contain warning', async () => {
      const mockChild = {
        kill: jest.fn()
      };
      
      mockExec.mockImplementation((command, options, callback) => {
        if (callback) callback(null, 'success', 'some error message');
        return mockChild as any;
      });

      await runParaCommand(['list']);
      expect(consoleSpy).toHaveBeenCalledWith('Para command warning: some error message');
    });

    test('should not log stderr that contains warning', async () => {
      const mockChild = {
        kill: jest.fn()
      };
      
      mockExec.mockImplementation((command, options, callback) => {
        if (callback) callback(null, 'success', 'warning: deprecated feature');
        return mockChild as any;
      });

      await runParaCommand(['list']);
      expect(consoleSpy).not.toHaveBeenCalled();
    });

    test('should handle empty stderr', async () => {
      const mockChild = {
        kill: jest.fn()
      };
      
      mockExec.mockImplementation((command, options, callback) => {
        if (callback) callback(null, 'success', '');
        return mockChild as any;
      });

      await runParaCommand(['list']);
      expect(consoleSpy).not.toHaveBeenCalled();
    });
  });

  describe('Error Handling', () => {
    test('should reject with McpError when command fails', async () => {
      const mockChild = {
        kill: jest.fn()
      };
      
      const testError = new Error('Command failed');
      mockExec.mockImplementation((command, options, callback) => {
        if (callback) callback(testError, '', '');
        return mockChild as any;
      });

      await expect(runParaCommand(['invalid-command'])).rejects.toEqual(
        new MockMcpError('InternalError', 'Para command failed: Command failed')
      );
    });

    test('should include original error message in McpError', async () => {
      const mockChild = {
        kill: jest.fn()
      };
      
      const testError = new Error('No such file or directory');
      mockExec.mockImplementation((command, options, callback) => {
        if (callback) callback(testError, '', '');
        return mockChild as any;
      });

      await expect(runParaCommand(['missing-command'])).rejects.toEqual(
        new MockMcpError('InternalError', 'Para command failed: No such file or directory')
      );
    });
  });

  describe('Timeout Handling', () => {
    test('should timeout after 30 seconds and kill process', async () => {
      jest.useFakeTimers();
      
      const mockChild = {
        kill: jest.fn()
      };
      
      mockExec.mockImplementation((command, options, callback) => {
        // Don't call callback to simulate hanging command
        return mockChild as any;
      });

      const promise = runParaCommand(['long-running-command']);
      
      // Fast-forward time by 30 seconds
      jest.advanceTimersByTime(30000);
      
      await expect(promise).rejects.toEqual(
        new MockMcpError('InternalError', 'Command timed out after 30 seconds: long-running-command')
      );
      
      expect(mockChild.kill).toHaveBeenCalledTimes(1);
      
      jest.useRealTimers();
    });

    test('should clear timeout when command completes successfully', async () => {
      jest.useFakeTimers();
      
      const mockChild = {
        kill: jest.fn()
      };
      
      mockExec.mockImplementation((command, options, callback) => {
        // Simulate command completing quickly
        setTimeout(() => {
          if (callback) callback(null, 'success', '');
        }, 1000);
        return mockChild as any;
      });

      const promise = runParaCommand(['quick-command']);
      
      // Fast-forward time by 1 second (command completes)
      jest.advanceTimersByTime(1000);
      
      const result = await promise;
      expect(result).toBe('success');
      
      // Fast-forward by 30 seconds to ensure timeout was cleared
      jest.advanceTimersByTime(30000);
      
      expect(mockChild.kill).not.toHaveBeenCalled();
      
      jest.useRealTimers();
    });

    test('should clear timeout when command fails', async () => {
      jest.useFakeTimers();
      
      const mockChild = {
        kill: jest.fn()
      };
      
      mockExec.mockImplementation((command, options, callback) => {
        // Simulate command failing quickly
        setTimeout(() => {
          if (callback) callback(new Error('Command failed'), '', '');
        }, 1000);
        return mockChild as any;
      });

      const promise = runParaCommand(['failing-command']);
      
      // Fast-forward time by 1 second (command fails)
      jest.advanceTimersByTime(1000);
      
      await expect(promise).rejects.toEqual(
        new MockMcpError('InternalError', 'Para command failed: Command failed')
      );
      
      // Fast-forward by 30 seconds to ensure timeout was cleared
      jest.advanceTimersByTime(30000);
      
      expect(mockChild.kill).not.toHaveBeenCalled();
      
      jest.useRealTimers();
    });
  });

  describe('Complex Command Scenarios', () => {
    test('should handle commands with multiple quoted arguments', async () => {
      const mockChild = {
        kill: jest.fn()
      };
      
      mockExec.mockImplementation((command, options, callback) => {
        expect(command).toBe('para dispatch "my session" "complex task with spaces" --file "path/to/file.md"');
        if (callback) callback(null, 'dispatched', '');
        return mockChild as any;
      });

      await runParaCommand(['dispatch', 'my session', 'complex task with spaces', '--file', 'path/to/file.md']);
      expect(mockExec).toHaveBeenCalledTimes(1);
    });

    test('should handle commands with mixed quoted and unquoted arguments', async () => {
      const mockChild = {
        kill: jest.fn()
      };
      
      mockExec.mockImplementation((command, options, callback) => {
        expect(command).toBe('para finish "commit message with spaces" --branch feature-branch');
        if (callback) callback(null, 'finished', '');
        return mockChild as any;
      });

      await runParaCommand(['finish', 'commit message with spaces', '--branch', 'feature-branch']);
      expect(mockExec).toHaveBeenCalledTimes(1);
    });
  });
});