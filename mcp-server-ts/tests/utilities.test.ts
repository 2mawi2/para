/**
 * Tests for utility functions in para-mcp-server
 * These tests focus on testing the core utility functions separately from the MCP protocol
 */

const { exec, execSync } = require('child_process');

// Mock child_process functions
jest.mock('child_process', () => ({
  exec: jest.fn(),
  execSync: jest.fn()
}));

const mockExec = exec as jest.MockedFunction<typeof exec>;
const mockExecSync = execSync as jest.MockedFunction<typeof execSync>;

describe('Para Binary Discovery', () => {
  let originalArgv: string[];
  let originalEnv: NodeJS.ProcessEnv;
  let originalCwd: () => string;

  beforeEach(() => {
    originalArgv = process.argv;
    originalEnv = process.env;
    originalCwd = process.cwd;
    jest.clearAllMocks();
  });

  afterEach(() => {
    process.argv = originalArgv;
    process.env = originalEnv;
    process.cwd = originalCwd;
  });

  test('should find homebrew para binary when MCP server is from homebrew', async () => {
    // Simulate running from homebrew
    process.argv = ['node', '/opt/homebrew/bin/para-mcp-server'];
    process.env = { ...originalEnv, HOME: '/Users/test' };
    
    // Mock successful execSync for homebrew location
    mockExecSync
      .mockImplementationOnce(() => { throw new Error('not found'); }) // First homebrew location fails
      .mockImplementationOnce(() => ''); // Second homebrew location succeeds

    // Import and test findParaBinary
    const { findParaBinary } = require('../src/test-utils');
    const result = findParaBinary();
    
    expect(mockExecSync).toHaveBeenCalledWith('test -x /opt/homebrew/bin/para', { stdio: 'ignore' });
    expect(mockExecSync).toHaveBeenCalledWith('test -x /usr/local/bin/para', { stdio: 'ignore' });
    expect(result).toBe('/usr/local/bin/para');
  });

  test('should fall back to development locations when not homebrew', async () => {
    process.argv = ['node', '/some/other/path/para-mcp-server'];
    process.env = { ...originalEnv, HOME: '/Users/test' };
    
    // Mock successful execSync for local development build
    mockExecSync
      .mockImplementationOnce(() => ''); // First location succeeds

    process.cwd = jest.fn().mockReturnValue('/project/root') as any;

    const { findParaBinary } = require('../src/test-utils');
    const result = findParaBinary();
    
    expect(mockExecSync).toHaveBeenCalledWith('test -x /project/root/target/release/para', { stdio: 'ignore' });
    expect(result).toBe('/project/root/target/release/para');
  });

  test('should return "para" as fallback when no binary found', async () => {
    process.argv = ['node', '/some/path/para-mcp-server'];
    process.env = { ...originalEnv };
    
    // Mock all execSync calls to fail
    mockExecSync.mockImplementation(() => { throw new Error('not found'); });

    const { findParaBinary } = require('../src/test-utils');
    const result = findParaBinary();
    
    expect(result).toBe('para');
  });
});

describe('Para Command Execution', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  test('should execute para command with proper argument quoting', async () => {
    const mockCallback = jest.fn((command: string, options: any, callback: any) => {
      expect(command).toContain('"session with spaces"');
      callback(null, 'success output', '');
    });
    mockExec.mockImplementation(mockCallback as any);

    const { runParaCommand } = require('../src/test-utils');
    const result = await runParaCommand(['start', 'session with spaces'], 'para');
    
    expect(result).toBe('success output');
    expect(mockCallback).toHaveBeenCalledWith(
      expect.stringContaining('"session with spaces"'),
      expect.any(Object),
      expect.any(Function)
    );
  });

  test('should set non-interactive environment variables', async () => {
    const mockCallback = jest.fn((command: string, options: any, callback: any) => {
      expect(options.env.PARA_NON_INTERACTIVE).toBe('1');
      expect(options.env.CI).toBe('1');
      callback(null, 'output', '');
    });
    mockExec.mockImplementation(mockCallback as any);

    const { runParaCommand } = require('../src/test-utils');
    await runParaCommand(['status'], 'para');
  });

  test('should handle command timeout after 30 seconds', async () => {
    const mockChild = {
      kill: jest.fn()
    };
    
    mockExec.mockImplementation(() => mockChild as any);

    const { runParaCommand } = require('../src/test-utils');
    
    jest.useFakeTimers();
    const promise = runParaCommand(['long-running-command'], 'para');
    
    // Fast-forward time to trigger timeout
    jest.advanceTimersByTime(31000);
    
    await expect(promise).rejects.toThrow('timed out after 30 seconds');
    expect(mockChild.kill).toHaveBeenCalled();
    
    jest.useRealTimers();
  });

  test('should handle para command errors', async () => {
    const mockCallback = jest.fn((command: string, options: any, callback: any) => {
      callback(new Error('Para command failed'), '', 'error output');
    });
    mockExec.mockImplementation(mockCallback as any);

    const { runParaCommand } = require('../src/test-utils');
    await expect(runParaCommand(['invalid-command'], 'para')).rejects.toThrow('Para command failed');
  });

  test('should handle stderr warnings without failing', async () => {
    const mockCallback = jest.fn((command: string, options: any, callback: any) => {
      callback(null, 'success', 'warning: something happened');
    });
    mockExec.mockImplementation(mockCallback as any);

    const { runParaCommand } = require('../src/test-utils');
    const result = await runParaCommand(['start', 'test'], 'para');
    expect(result).toBe('success');
  });
});