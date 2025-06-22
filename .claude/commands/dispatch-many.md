Your task is to parse the user's request and dispatch a separate `para` agent for each distinct task identified.

**User's Request:**
"$ARGUMENTS"

**Execution Plan:**
1.  Identify each individual task from the user's request.
2.  For each identified task, use your internal `Task` tool to run the following process in parallel:

> **Sub-Task Prompt Template:**
> "You are a sub-agent. Your only job is to create and dispatch a single `para` session for the following task: `{{task_description}}`.
> First, create a new task file named `tasks/TASK_{{sanitized_task_description}}.md`. The content of this file must be a detailed and actionable task based on the provided description.
> Second, after the task file is successfully written, use the `para` MCP tool to dispatch a new agent to work on it. The command must use the `--file` argument pointing to the new task file and include the `--dangerously-skip-permissions` flag.
> Report back with the name of the session you created."

3. After all parallel dispatch tasks have been started, provide a summary list of the sessions that were successfully created.