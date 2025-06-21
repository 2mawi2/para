/**
 * Para Binary Interface - Handles discovery and execution of the para binary
 */

import { exec, execSync } from "child_process";
import { promisify } from "util";
import { McpError, ErrorCode } from "@modelcontextprotocol/sdk/types.js";

const execAsync = promisify(exec);

export interface ExecResult {
  stdout: string;
  stderr: string;
}

export class ParaBinaryInterface {
  private binaryPath: string;

  constructor() {
    this.binaryPath = this.findBinary();
    console.error(`Para MCP server using para binary: ${this.binaryPath}`);
  }

  /**
   * Dynamically discover the para binary path
   */
  private findBinary(): string {
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

  /**
   * Check if the para binary exists and is executable
   */
  public validateBinaryExists(): boolean {
    try {
      execSync(`test -x ${this.binaryPath}`, { stdio: 'ignore' });
      return true;
    } catch {
      return false;
    }
  }

  /**
   * Get the current binary path
   */
  public getBinaryPath(): string {
    return this.binaryPath;
  }

  /**
   * Execute a para command with the given arguments
   */
  public async executeCommand(command: string, args: string[]): Promise<ExecResult> {
    return new Promise((resolve, reject) => {
      // Properly quote arguments that contain spaces
      const quotedArgs = args.map(arg => {
        // If the argument contains spaces and isn't already quoted, wrap it in quotes
        if (arg.includes(' ') && !arg.startsWith('"') && !arg.startsWith("'")) {
          return `"${arg.replace(/"/g, '\\"')}"`;
        }
        return arg;
      });

      const fullCommand = `${this.binaryPath} ${command} ${quotedArgs.join(' ')}`;
      
      // Set environment to indicate non-interactive mode
      const env = {
        ...process.env,
        PARA_NON_INTERACTIVE: '1',
        CI: '1'  // Many CLIs respect this as well
      };

      const child = exec(fullCommand, { env }, (error, stdout, stderr) => {
        clearTimeout(timeout);
        
        if (error) {
          reject(new McpError(ErrorCode.InternalError, `Para command failed: ${error.message}`));
          return;
        }
        
        if (stderr && !stderr.includes('warning')) {
          console.error(`Para command warning: ${stderr}`);
        }
        
        resolve({
          stdout: stdout.trim(),
          stderr: stderr.trim()
        });
      });

      // Set a 30-second timeout
      const timeout = setTimeout(() => {
        child.kill();
        reject(new McpError(ErrorCode.InternalError, `Command timed out after 30 seconds: ${command} ${args.join(' ')}`));
      }, 30000);
    });
  }
}