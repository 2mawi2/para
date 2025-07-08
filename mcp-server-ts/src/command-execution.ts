import { exec } from "child_process";
import { McpError, ErrorCode } from "@modelcontextprotocol/sdk/types.js";

// Abstracts command execution to handle timeouts, environment setup, and error handling
export async function runParaCommand(paraBinary: string, args: string[]): Promise<string> {
  return new Promise((resolve, reject) => {
    // Prevent shell injection and argument splitting issues
    const quotedArgs = args.map(arg => {
      if (arg.includes(' ') && !arg.startsWith('"') && !arg.startsWith("'")) {
        return `"${arg.replace(/"/g, '\\"')}"`;
      }
      return arg;
    });

    const command = `${paraBinary} ${quotedArgs.join(' ')}`;
    
    // Prevent para from blocking on user prompts in automated contexts
    const env = {
      ...process.env,
      PARA_NON_INTERACTIVE: '1',
      CI: '1'  // Many CLIs respect this as well
    };

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

    // Prevent hanging on complex operations while allowing time for worktree setup
    const timeout = setTimeout(() => {
      child.kill();
      reject(new McpError(ErrorCode.InternalError, `Command timed out after 30 seconds: ${args.join(' ')}`));
    }, 30000);
  });
}