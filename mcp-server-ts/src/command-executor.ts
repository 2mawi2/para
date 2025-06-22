/**
 * Para Command Executor Module
 * 
 * Handles execution of para commands with proper argument quoting,
 * environment setup, timeout handling, and error management.
 * Extracted from the monolithic para-mcp-server.ts for better modularity.
 */

import { exec } from "child_process";
import { McpError, ErrorCode } from "@modelcontextprotocol/sdk/types.js";

export class ParaCommandExecutor {
  private readonly binaryPath: string;
  private readonly timeoutMs: number;

  constructor(binaryPath: string, timeoutMs: number = 30000) {
    this.binaryPath = binaryPath;
    this.timeoutMs = timeoutMs;
  }

  /**
   * Execute a para command with the given arguments
   * @param args Command arguments
   * @returns Promise resolving to command output
   */
  public async execute(args: string[]): Promise<string> {
    return new Promise((resolve, reject) => {
      const quotedArgs = this.formatCommand(args);
      const command = `${this.binaryPath} ${quotedArgs.join(' ')}`;
      
      const env = this.createEnvironment();
      const child = exec(command, { env }, (error, stdout, stderr) => {
        clearTimeout(timeout);
        
        if (error) {
          reject(new McpError(ErrorCode.InternalError, `Para command failed: ${error.message}`));
          return;
        }
        
        this.handleStderr(stderr);
        resolve(stdout.trim());
      });

      const timeout = this.createTimeout(child, args);
    });
  }

  /**
   * Properly quote arguments that contain spaces
   * @param args Raw command arguments
   * @returns Quoted arguments ready for shell execution
   */
  private formatCommand(args: string[]): string[] {
    return args.map(arg => {
      // If the argument contains spaces and isn't already quoted, wrap it in quotes
      if (arg.includes(' ') && !arg.startsWith('"') && !arg.startsWith("'")) {
        return `"${arg.replace(/"/g, '\\"')}"`;
      }
      return arg;
    });
  }

  /**
   * Create environment variables for non-interactive mode
   * @returns Environment object with para-specific variables
   */
  private createEnvironment(): NodeJS.ProcessEnv {
    return {
      ...process.env,
      PARA_NON_INTERACTIVE: '1',
      CI: '1'  // Many CLIs respect this as well
    };
  }

  /**
   * Handle stderr output from para commands
   * @param stderr Standard error output
   */
  private handleStderr(stderr: string): void {
    if (stderr && !stderr.includes('warning')) {
      console.error(`Para command warning: ${stderr}`);
    }
  }

  /**
   * Create timeout handler for command execution
   * @param child Child process
   * @param args Command arguments for error message
   * @returns Timeout ID
   */
  private createTimeout(child: any, args: string[]): NodeJS.Timeout {
    return setTimeout(() => {
      child.kill();
      throw new McpError(
        ErrorCode.InternalError, 
        `Command timed out after ${this.timeoutMs / 1000} seconds: ${args.join(' ')}`
      );
    }, this.timeoutMs);
  }
}