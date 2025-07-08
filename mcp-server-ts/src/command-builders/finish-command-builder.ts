import { ParaFinishArgs } from '../types';

export class FinishCommandBuilder {
  static build(args: ParaFinishArgs): string[] {
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
}