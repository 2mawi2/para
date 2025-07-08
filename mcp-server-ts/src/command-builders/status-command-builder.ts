import { ParaStatusShowArgs } from '../types';

export class StatusCommandBuilder {
  static build(args: ParaStatusShowArgs): string[] {
    const cmdArgs = ["status", "show"];
    if (args.session) {
      cmdArgs.push(args.session);
    }
    if (args.json) {
      cmdArgs.push("--json");
    }
    return cmdArgs;
  }
}