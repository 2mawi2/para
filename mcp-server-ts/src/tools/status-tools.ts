/**
 * Status Tools - Handle para status and listing operations
 */

import { ParaBinaryInterface, ExecResult } from "../para/binary-interface.js";

export class StatusTools {
  constructor(private binaryInterface: ParaBinaryInterface) {}

  /**
   * List para sessions
   */
  async list(args: { verbose?: boolean; archived?: boolean; quiet?: boolean }): Promise<ExecResult> {
    const command = "list";
    const cmdArgs: string[] = [];
    
    if (args.verbose) {
      cmdArgs.push("--verbose");
    }
    
    if (args.archived) {
      cmdArgs.push("--archived");
    }
    
    if (args.quiet) {
      cmdArgs.push("--quiet");
    }
    
    return await this.binaryInterface.executeCommand(command, cmdArgs);
  }

  /**
   * Show status of para sessions
   */
  async statusShow(args: { session?: string; json?: boolean }): Promise<ExecResult> {
    const command = "status";
    const cmdArgs: string[] = ["show"];
    
    if (args.session) {
      cmdArgs.push(args.session);
    }
    
    if (args.json) {
      cmdArgs.push("--json");
    }
    
    return await this.binaryInterface.executeCommand(command, cmdArgs);
  }
}