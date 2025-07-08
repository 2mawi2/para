import { ParaResumeArgs } from '../types';

export class ResumeCommandBuilder {
  static build(args: ParaResumeArgs): string[] {
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
    if (args.dangerously_skip_permissions) {
      cmdArgs.push("--dangerously-skip-permissions");
    }
    if (args.sandbox) {
      cmdArgs.push("--sandbox");
    }
    if (args.no_sandbox) {
      cmdArgs.push("--no-sandbox");
    }
    if (args.sandbox_profile) {
      cmdArgs.push("--sandbox-profile", args.sandbox_profile);
    }
    
    return cmdArgs;
  }
}