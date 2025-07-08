import { ParaStartArgs } from '../types';

export class StartCommandBuilder {
  static build(args: ParaStartArgs): string[] {
    const cmdArgs = ["start"];
    
    // Handle unified start arguments
    if (args.name_or_session) {
      cmdArgs.push(args.name_or_session);
    }
    if (args.prompt) {
      cmdArgs.push(args.prompt);
    }
    if (args.file) {
      cmdArgs.push("--file", args.file);
    }
    if (args.dangerously_skip_permissions) {
      cmdArgs.push("--dangerously-skip-permissions");
    }
    if (args.container) {
      cmdArgs.push("--container");
    }
    if (args.docker_args && args.docker_args.length > 0) {
      cmdArgs.push("--docker-args", ...args.docker_args);
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
    if (args.docker_image) {
      cmdArgs.push("--docker-image", args.docker_image);
    }
    if (args.allow_domains) {
      cmdArgs.push("--allow-domains", args.allow_domains);
    }
    if (args.no_forward_keys) {
      cmdArgs.push("--no-forward-keys");
    }
    if (args.setup_script) {
      cmdArgs.push("--setup-script", args.setup_script);
    }
    
    return cmdArgs;
  }
}