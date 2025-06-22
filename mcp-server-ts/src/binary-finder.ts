/**
 * Para Binary Discovery Module
 * 
 * Handles finding the para binary in various system locations.
 * Extracted from the monolithic para-mcp-server.ts for better modularity.
 */

import { execSync } from "child_process";

export class ParaBinaryFinder {
  /**
   * Find the para binary from homebrew locations
   * @returns Path to homebrew para binary or null if not found
   */
  private static findHomebrewBinary(): string | null {
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
    
    return null;
  }

  /**
   * Find the para binary from development build locations
   * @returns Path to development para binary or null if not found
   */
  private static findDevelopmentBinary(): string | null {
    const developmentLocations = [
      process.cwd() + "/target/release/para",     // Local development build
      process.cwd() + "/target/debug/para",       // Local debug build
    ];

    for (const location of developmentLocations) {
      try {
        execSync(`test -x ${location}`, { stdio: 'ignore' });
        return location;
      } catch {
        // Continue to next location
      }
    }

    return null;
  }

  /**
   * Find the para binary from system locations
   * @returns Path to system para binary or null if not found
   */
  private static findSystemBinary(): string | null {
    const systemLocations = [
      process.env.HOME + "/.local/bin/para",       // Local installation
      "/opt/homebrew/bin/para",                    // Homebrew fallback
      "/usr/local/bin/para",                       // Homebrew fallback
    ];

    for (const location of systemLocations) {
      // Skip if HOME is undefined
      if (location.includes("undefined")) {
        continue;
      }
      
      try {
        execSync(`test -x ${location}`, { stdio: 'ignore' });
        return location;
      } catch {
        // Continue to next location
      }
    }

    return null;
  }

  /**
   * Check if the MCP server is running from homebrew
   * @returns True if running from homebrew installation
   */
  private static isHomebrewMcp(): boolean {
    const mcpPath = process.argv[1]; // Path to this script
    return Boolean(mcpPath && (mcpPath.includes('/homebrew/') || mcpPath.includes('/usr/local/')));
  }

  /**
   * Find the para binary, checking locations in order of preference
   * @returns Path to para binary (fallback to "para" if not found)
   */
  public static findBinary(): string {
    const isHomebrewMcp = this.isHomebrewMcp();
    
    if (isHomebrewMcp) {
      // For homebrew MCP server, only use homebrew para
      const homebrewBinary = this.findHomebrewBinary();
      if (homebrewBinary) {
        return homebrewBinary;
      }
      
      // If homebrew MCP but no homebrew para found, there's a problem
      console.error("Warning: Homebrew MCP server but para binary not found in homebrew locations");
    }
    
    // For development or other installations, check in order
    const developmentBinary = this.findDevelopmentBinary();
    if (developmentBinary) {
      return developmentBinary;
    }

    const systemBinary = this.findSystemBinary();
    if (systemBinary) {
      return systemBinary;
    }

    // Fallback to 'para' in PATH
    return "para";
  }
}