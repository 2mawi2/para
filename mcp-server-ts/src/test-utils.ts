/**
 * Test utilities for extracting functions from the monolithic para-mcp-server.ts
 * This allows us to test individual functions before refactoring
 */

const { exec, execSync } = require("child_process");
const { promisify } = require("util");
const { McpError, ErrorCode } = require("@modelcontextprotocol/sdk/types.js");

const execAsync = promisify(exec);

// Extract the findParaBinary function for testing
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

// Extract the runParaCommand function for testing
async function runParaCommand(args: string[], paraBinary: string = "para"): Promise<string> {
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
    
    const child = exec(command, { env }, (error: any, stdout: any, stderr: any) => {
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

// Extract tool argument building logic for testing
function buildParaStartArgs(args: any): string[] {
  const cmdArgs = ["start"];
  if (args.session_name) {
    cmdArgs.push(args.session_name);
  }
  if (args.dangerously_skip_permissions) {
    cmdArgs.push("--dangerously-skip-permissions");
  }
  return cmdArgs;
}

function buildParaFinishArgs(args: any): string[] {
  const cmdArgs = ["finish"];
  cmdArgs.push(args.commit_message);
  if (args.session) {
    cmdArgs.push(args.session);
  }
  if (args.branch) {
    cmdArgs.push("--branch", args.branch);
  }
  return cmdArgs;
}

function buildParaDispatchArgs(args: any): string[] {
  const cmdArgs = ["dispatch"];
  cmdArgs.push(args.session_name);

  if (args.file) {
    cmdArgs.push("--file", args.file);
  } else if (args.task_description) {
    cmdArgs.push(args.task_description);
  }

  if (args.dangerously_skip_permissions) {
    cmdArgs.push("--dangerously-skip-permissions");
  }

  return cmdArgs;
}

function buildParaListArgs(args: any): string[] {
  const cmdArgs = ["list"];
  if (args.verbose) {
    cmdArgs.push("--verbose");
  }
  if (args.archived) {
    cmdArgs.push("--archived");
  }
  if (args.quiet) {
    cmdArgs.push("--quiet");
  }
  return cmdArgs;
}

function buildParaRecoverArgs(args: any): string[] {
  const cmdArgs = ["recover"];
  if (args.session_name) {
    cmdArgs.push(args.session_name);
  }
  return cmdArgs;
}

function buildParaResumeArgs(args: any): string[] {
  const cmdArgs = ["resume"];
  if (args.session) {
    cmdArgs.push(args.session);
  }
  if (args.prompt) {
    cmdArgs.push("--prompt", args.prompt);
  }
  if (args.file) {
    cmdArgs.push("--file", args.file);
  }
  return cmdArgs;
}

function buildParaCancelArgs(args: any): string[] {
  const cmdArgs = ["cancel"];
  if (args.session_name) {
    cmdArgs.push(args.session_name);
  }
  if (args.force) {
    cmdArgs.push("--force");
  }
  return cmdArgs;
}

function buildParaStatusArgs(args: any): string[] {
  const cmdArgs = ["status", "show"];
  if (args.session) {
    cmdArgs.push(args.session);
  }
  if (args.json) {
    cmdArgs.push("--json");
  }
  return cmdArgs;
}

module.exports = {
  findParaBinary,
  runParaCommand,
  buildParaStartArgs,
  buildParaFinishArgs,
  buildParaDispatchArgs,
  buildParaListArgs,
  buildParaRecoverArgs,
  buildParaResumeArgs,
  buildParaCancelArgs,
  buildParaStatusArgs
};