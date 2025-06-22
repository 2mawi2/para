#!/usr/bin/env node
/**
 * Para Command Execution Utilities
 * Handles executing para commands with proper argument quoting and error handling
 */

import { exec } from "child_process";
import { McpError, ErrorCode } from "@modelcontextprotocol/sdk/types.js";

/**
 * Executes a para command with proper argument quoting and timeout handling
 * @param args Command arguments to pass to para
 * @param paraBinary Path to the para binary
 * @returns Promise that resolves to the command output
 */
export async function runParaCommand(args: string[], paraBinary: string): Promise<string> {
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

    // Set a 30-second timeout  
    let timeout: NodeJS.Timeout;
    
    const child = exec(command, { env }, (error, stdout, stderr) => {
      clearTimeout(timeout);
      
      if (error) {
        reject(new McpError(ErrorCode.InternalError, `Para command failed: ${error.message}`));
        return;
      }
      
      if (stderr && !stderr.includes('warning')) {
        console.error(`Para command warning: ${stderr}`);
      }
      
      resolve(stdout.trim());
    });

    timeout = setTimeout(() => {
      child.kill();
      reject(new McpError(ErrorCode.InternalError, `Command timed out after 30 seconds: ${args.join(' ')}`));
    }, 30000);
  });
}