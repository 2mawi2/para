/**
 * Tests for para binary discovery functionality
 * These tests verify the logic for finding the para binary in various locations
 */

import { execSync } from 'child_process';
import { jest } from '@jest/globals';

// Mock child_process module
jest.mock('child_process', () => ({
  execSync: jest.fn(),
  exec: jest.fn()
}));

const mockExecSync = execSync as jest.MockedFunction<typeof execSync>;

// We need to extract the binary discovery logic for testing
// This represents the current logic from para-mcp-server.ts lines 23-70
function findParaBinary(): string {
  // Check if MCP server is running from homebrew
  const mcpPath = process.argv[1]; // Path to this script
  const isHomebrewMcp = mcpPath && (mcpPath.includes('/homebrew/') || mcpPath.includes('/usr/local/'));
  
  if (isHomebrewMcp) {
    // For homebrew MCP server, only use homebrew para
    const homebrewLocations = [
      "/opt/homebrew/bin/para",              // Apple Silicon
      "/usr/local/bin/para",                 // Intel Mac
      "/home/linuxbrew/.linuxbrew/bin/para", // Linux
    ];
    
    for (const location of homebrewLocations) {
      try {
        execSync(`test -x ${location}`, { stdio: 'ignore' });
        return location;
      } catch {
        // Continue to next location
      }
    }
    
    // If homebrew MCP but no homebrew para found, there's a problem
    console.error("Warning: Homebrew MCP server but para binary not found in homebrew locations");
  }
  
  // For development or other installations, check in order
  const locations = [
    process.cwd() + "/target/release/para",           // Local development build
    process.cwd() + "/target/debug/para",             // Local debug build
    process.env.HOME + "/.local/bin/para",           // Local installation
    "/opt/homebrew/bin/para",                        // Homebrew fallback
    "/usr/local/bin/para",                           // Homebrew fallback
    "para"                                           // System PATH
  ];

  for (const location of locations) {
    try {
      execSync(`test -x ${location}`, { stdio: 'ignore' });
      return location;
    } catch {
      // Continue to next location
    }
  }

  // Fallback to 'para' in PATH
  return "para";
}

describe('Binary Discovery', () => {
  let originalArgv: string[];
  let originalCwd: string;
  let originalHome: string | undefined;

  beforeEach(() => {
    // Store original values
    originalArgv = [...process.argv];
    originalCwd = process.cwd();
    originalHome = process.env.HOME;
    
    // Clear mock calls
    jest.clearAllMocks();
  });

  afterEach(() => {
    // Restore original values
    process.argv = originalArgv;
    Object.defineProperty(process, 'cwd', {
      value: () => originalCwd
    });
    process.env.HOME = originalHome;
  });

  describe('Homebrew MCP Detection', () => {
    test('should detect homebrew MCP from path containing /homebrew/', () => {
      process.argv[1] = '/opt/homebrew/lib/node_modules/para-mcp-server/build/para-mcp-server.js';
      
      mockExecSync
        .mockImplementationOnce(() => { throw new Error('not found'); }) // First homebrew location fails
        .mockImplementationOnce(() => ''); // Second homebrew location succeeds
      
      const result = findParaBinary();
      
      expect(result).toBe('/usr/local/bin/para');
      expect(mockExecSync).toHaveBeenCalledWith('test -x /opt/homebrew/bin/para', { stdio: 'ignore' });
      expect(mockExecSync).toHaveBeenCalledWith('test -x /usr/local/bin/para', { stdio: 'ignore' });
    });

    test('should detect homebrew MCP from path containing /usr/local/', () => {
      process.argv[1] = '/usr/local/lib/node_modules/para-mcp-server/build/para-mcp-server.js';
      
      mockExecSync.mockImplementationOnce(() => ''); // First homebrew location succeeds
      
      const result = findParaBinary();
      
      expect(result).toBe('/opt/homebrew/bin/para');
      expect(mockExecSync).toHaveBeenCalledWith('test -x /opt/homebrew/bin/para', { stdio: 'ignore' });
    });

    test('should try all homebrew locations when first ones fail', () => {
      process.argv[1] = '/opt/homebrew/lib/node_modules/para-mcp-server/build/para-mcp-server.js';
      
      mockExecSync
        .mockImplementationOnce(() => { throw new Error('not found'); }) // Apple Silicon fails
        .mockImplementationOnce(() => { throw new Error('not found'); }) // Intel Mac fails  
        .mockImplementationOnce(() => ''); // Linux succeeds
      
      const result = findParaBinary();
      
      expect(result).toBe('/home/linuxbrew/.linuxbrew/bin/para');
      expect(mockExecSync).toHaveBeenCalledTimes(3);
    });

    test('should warn and continue when homebrew MCP but no homebrew para found', () => {
      process.argv[1] = '/opt/homebrew/lib/node_modules/para-mcp-server/build/para-mcp-server.js';
      const consoleSpy = jest.spyOn(console, 'error').mockImplementation(() => {});
      
      // All homebrew locations fail
      mockExecSync.mockImplementation(() => { throw new Error('not found'); });
      
      findParaBinary();
      
      expect(consoleSpy).toHaveBeenCalledWith("Warning: Homebrew MCP server but para binary not found in homebrew locations");
      
      consoleSpy.mockRestore();
    });
  });

  describe('Development and System Installation Detection', () => {
    test('should find local development release build first', () => {
      process.argv[1] = '/home/user/para/mcp-server-ts/build/para-mcp-server.js'; // Non-homebrew path
      Object.defineProperty(process, 'cwd', {
        value: () => '/home/user/para'
      });
      
      mockExecSync.mockImplementationOnce(() => ''); // Release build found
      
      const result = findParaBinary();
      
      expect(result).toBe('/home/user/para/target/release/para');
      expect(mockExecSync).toHaveBeenCalledWith('test -x /home/user/para/target/release/para', { stdio: 'ignore' });
    });

    test('should find local development debug build when release not available', () => {
      process.argv[1] = '/home/user/para/mcp-server-ts/build/para-mcp-server.js';
      Object.defineProperty(process, 'cwd', {
        value: () => '/home/user/para'
      });
      
      mockExecSync
        .mockImplementationOnce(() => { throw new Error('not found'); }) // Release build fails
        .mockImplementationOnce(() => ''); // Debug build succeeds
      
      const result = findParaBinary();
      
      expect(result).toBe('/home/user/para/target/debug/para');
    });

    test('should find local user installation', () => {
      process.argv[1] = '/some/other/path/para-mcp-server.js';
      process.env.HOME = '/home/testuser';
      
      mockExecSync
        .mockImplementationOnce(() => { throw new Error('not found'); }) // Release build fails
        .mockImplementationOnce(() => { throw new Error('not found'); }) // Debug build fails
        .mockImplementationOnce(() => Buffer.from('')); // Local installation succeeds
      
      const result = findParaBinary();
      
      expect(result).toBe('/home/testuser/.local/bin/para');
    });

    test('should fallback to system PATH when no specific location found', () => {
      process.argv[1] = '/some/other/path/para-mcp-server.js';
      
      // All specific locations fail
      mockExecSync.mockImplementation(() => { throw new Error('not found'); });
      
      const result = findParaBinary();
      
      expect(result).toBe('para');
    });
  });

  describe('Error Handling', () => {
    test('should handle execSync exceptions gracefully', () => {
      process.argv[1] = '/some/path/para-mcp-server.js';
      
      mockExecSync.mockImplementation(() => { 
        throw new Error('Permission denied'); 
      });
      
      expect(() => findParaBinary()).not.toThrow();
      const result = findParaBinary();
      expect(result).toBe('para');
    });

    test('should handle missing HOME environment variable', () => {
      process.argv[1] = '/some/path/para-mcp-server.js';
      delete process.env.HOME;
      
      mockExecSync
        .mockImplementationOnce(() => { throw new Error('not found'); }) // Release build fails
        .mockImplementationOnce(() => { throw new Error('not found'); }) // Debug build fails
        .mockImplementationOnce(() => { throw new Error('not found'); }) // Local installation fails (HOME undefined)
        .mockImplementationOnce(() => Buffer.from('')); // Homebrew fallback succeeds
      
      const result = findParaBinary();
      
      expect(result).toBe('/opt/homebrew/bin/para');
    });
  });

  describe('Path Priority Order', () => {
    test('should check locations in correct priority order for non-homebrew MCP', () => {
      process.argv[1] = '/some/path/para-mcp-server.js';
      Object.defineProperty(process, 'cwd', {
        value: () => '/test/dir'
      });
      process.env.HOME = '/home/user';
      
      // All locations fail to test the order
      mockExecSync.mockImplementation(() => { throw new Error('not found'); });
      
      findParaBinary();
      
      const expectedCalls = [
        'test -x /test/dir/target/release/para',     // Local development build
        'test -x /test/dir/target/debug/para',       // Local debug build  
        'test -x /home/user/.local/bin/para',        // Local installation
        'test -x /opt/homebrew/bin/para',            // Homebrew fallback
        'test -x /usr/local/bin/para',               // Homebrew fallback
        'test -x para'                               // System PATH
      ];
      
      expectedCalls.forEach((expectedCall, index) => {
        expect(mockExecSync).toHaveBeenNthCalledWith(index + 1, expectedCall, { stdio: 'ignore' });
      });
    });
  });
});