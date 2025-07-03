import { exec, execSync } from "child_process";
import { McpError, ErrorCode } from "@modelcontextprotocol/sdk/types.js";

/**
 * Dynamic discovery needed to support homebrew, dev builds, and system installations
 */
export function findParaBinary(): string {
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
 * Abstracts command execution to handle timeouts, environment setup, and error handling
 */
export async function runParaCommand(args: string[], paraBinary: string): Promise<string> {
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