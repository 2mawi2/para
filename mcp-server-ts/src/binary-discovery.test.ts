import { execSync } from 'child_process';
import { findParaBinary } from './binary-discovery';

// Prevent actual binary execution during tests
jest.mock('child_process', () => ({
  execSync: jest.fn(),
}));

const mockExecSync = execSync as jest.MockedFunction<typeof execSync>;

describe('Binary Discovery', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    // Prevent tests from interfering with each other's binary discovery
    process.argv = ['node', 'test-script.js'];
  });

  test('should find para binary in local development build', () => {
    mockExecSync.mockImplementation((command: string) => {
      if (command.includes('test -x') && command.includes('target/release/para')) {
        return Buffer.from('');
      }
      throw new Error('Command failed');
    });

    const binary = findParaBinary();
    expect(binary).toContain('target/release/para');
  });

  test('should find para binary in local debug build', () => {
    mockExecSync.mockImplementation((command: string) => {
      if (command.includes('test -x') && command.includes('target/debug/para')) {
        return Buffer.from('');
      }
      throw new Error('Command failed');
    });

    const binary = findParaBinary();
    expect(binary).toContain('target/debug/para');
  });

  test('should find para binary in homebrew installation', () => {
    // Simulate homebrew installation environment for path testing
    const originalArgv = process.argv[1];
    process.argv[1] = '/opt/homebrew/bin/para-mcp-server';
    
    mockExecSync.mockImplementation((command: string) => {
      if (command.includes('test -x') && command.includes('/opt/homebrew/bin/para')) {
        return Buffer.from('');
      }
      throw new Error('Command failed');
    });

    const binary = findParaBinary();
    expect(binary).toBe('/opt/homebrew/bin/para');
    
    // Prevent test state from affecting other tests
    process.argv[1] = originalArgv;
  });

  test('should fallback to PATH para when no specific location found', () => {
    mockExecSync.mockImplementation(() => {
      throw new Error('Command failed');
    });

    const binary = findParaBinary();
    expect(binary).toBe('para');
  });
});