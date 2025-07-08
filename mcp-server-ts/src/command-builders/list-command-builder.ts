import { ParaListArgs } from '../types';

export class ListCommandBuilder {
  static build(args: ParaListArgs): string[] {
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
}