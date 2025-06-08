use crate::utils::{ParaError, Result};
use serde_json::{Map, Value};

pub fn json_escape_string(input: &str) -> String {
    input
        .chars()
        .map(|c| match c {
            '"' => "\\\"".to_string(),
            '\\' => "\\\\".to_string(),
            '\n' => "\\n".to_string(),
            '\r' => "\\r".to_string(),
            '\t' => "\\t".to_string(),
            '\u{08}' => "\\b".to_string(),
            '\u{0C}' => "\\f".to_string(),
            c if c.is_control() => format!("\\u{:04x}", c as u32),
            c => c.to_string(),
        })
        .collect()
}

pub fn create_vscode_task(label: &str, command: &str, args: &[&str]) -> Value {
    let mut task = Map::new();

    task.insert("label".to_string(), Value::String(label.to_string()));
    task.insert("type".to_string(), Value::String("shell".to_string()));
    task.insert("command".to_string(), Value::String(command.to_string()));

    if !args.is_empty() {
        let args_array: Vec<Value> = args
            .iter()
            .map(|arg| Value::String(arg.to_string()))
            .collect();
        task.insert("args".to_string(), Value::Array(args_array));
    }

    let mut group = Map::new();
    group.insert("kind".to_string(), Value::String("build".to_string()));
    group.insert("isDefault".to_string(), Value::Bool(false));
    task.insert("group".to_string(), Value::Object(group));

    let mut presentation = Map::new();
    presentation.insert("echo".to_string(), Value::Bool(true));
    presentation.insert("reveal".to_string(), Value::String("always".to_string()));
    presentation.insert("focus".to_string(), Value::Bool(false));
    presentation.insert("panel".to_string(), Value::String("shared".to_string()));
    presentation.insert("showReuseMessage".to_string(), Value::Bool(true));
    presentation.insert("clear".to_string(), Value::Bool(false));
    task.insert("presentation".to_string(), Value::Object(presentation));

    let mut options = Map::new();
    options.insert(
        "cwd".to_string(),
        Value::String("${workspaceFolder}".to_string()),
    );
    task.insert("options".to_string(), Value::Object(options));

    task.insert("problemMatcher".to_string(), Value::Array(vec![]));
    task.insert("runOptions".to_string(), Value::Object(Map::new()));

    Value::Object(task)
}

pub fn create_cursor_task(label: &str, command: &str, args: &[&str]) -> Value {
    create_vscode_task(label, command, args)
}

pub fn create_tasks_json(tasks: Vec<Value>) -> Value {
    let mut tasks_json = Map::new();
    tasks_json.insert("version".to_string(), Value::String("2.0.0".to_string()));
    tasks_json.insert("tasks".to_string(), Value::Array(tasks));

    Value::Object(tasks_json)
}

pub fn create_claude_task(prompt: &str, working_dir: Option<&str>) -> Value {
    let mut task = Map::new();

    task.insert(
        "label".to_string(),
        Value::String("Para Session".to_string()),
    );
    task.insert("type".to_string(), Value::String("shell".to_string()));
    task.insert("command".to_string(), Value::String("claude".to_string()));

    let mut args = vec!["--prompt".to_string(), json_escape_string(prompt)];
    if let Some(dir) = working_dir {
        args.extend(vec!["--cwd".to_string(), dir.to_string()]);
    }

    let args_array: Vec<Value> = args.iter().map(|arg| Value::String(arg.clone())).collect();
    task.insert("args".to_string(), Value::Array(args_array));

    let mut group = Map::new();
    group.insert("kind".to_string(), Value::String("build".to_string()));
    group.insert("isDefault".to_string(), Value::Bool(true));
    task.insert("group".to_string(), Value::Object(group));

    let mut presentation = Map::new();
    presentation.insert("echo".to_string(), Value::Bool(false));
    presentation.insert("reveal".to_string(), Value::String("always".to_string()));
    presentation.insert("focus".to_string(), Value::Bool(true));
    presentation.insert("panel".to_string(), Value::String("new".to_string()));
    presentation.insert("showReuseMessage".to_string(), Value::Bool(false));
    presentation.insert("clear".to_string(), Value::Bool(true));
    task.insert("presentation".to_string(), Value::Object(presentation));

    let mut options = Map::new();
    if let Some(dir) = working_dir {
        options.insert("cwd".to_string(), Value::String(dir.to_string()));
    } else {
        options.insert(
            "cwd".to_string(),
            Value::String("${workspaceFolder}".to_string()),
        );
    }
    task.insert("options".to_string(), Value::Object(options));

    task.insert("problemMatcher".to_string(), Value::Array(vec![]));
    task.insert("runOptions".to_string(), Value::Object(Map::new()));

    Value::Object(task)
}

pub fn pretty_print_json(value: &Value) -> Result<String> {
    serde_json::to_string_pretty(value).map_err(ParaError::from)
}

pub fn minify_json(value: &Value) -> Result<String> {
    serde_json::to_string(value).map_err(ParaError::from)
}

pub fn merge_json_objects(base: &mut Value, overlay: &Value) -> Result<()> {
    match (base, overlay) {
        (Value::Object(base_map), Value::Object(overlay_map)) => {
            for (key, value) in overlay_map {
                if let Some(base_value) = base_map.get_mut(key) {
                    merge_json_objects(base_value, value)?;
                } else {
                    base_map.insert(key.clone(), value.clone());
                }
            }
        }
        (base_value, overlay_value) => {
            *base_value = overlay_value.clone();
        }
    }
    Ok(())
}

pub fn extract_string_from_json(value: &Value, key: &str) -> Option<String> {
    value.get(key)?.as_str().map(|s| s.to_string())
}

pub fn extract_array_from_json<'a>(value: &'a Value, key: &str) -> Option<&'a Vec<Value>> {
    value.get(key)?.as_array()
}

pub fn extract_object_from_json<'a>(value: &'a Value, key: &str) -> Option<&'a Map<String, Value>> {
    value.get(key)?.as_object()
}

pub fn validate_json_structure(value: &Value, required_keys: &[&str]) -> Result<()> {
    if let Value::Object(obj) = value {
        for key in required_keys {
            if !obj.contains_key(*key) {
                return Err(ParaError::config_error(format!(
                    "Missing required key '{}' in JSON object",
                    key
                )));
            }
        }
        Ok(())
    } else {
        Err(ParaError::config_error("Expected JSON object"))
    }
}

pub fn create_workspace_settings(settings: &Map<String, Value>) -> Value {
    let mut workspace = Map::new();

    for (key, value) in settings {
        workspace.insert(key.clone(), value.clone());
    }

    Value::Object(workspace)
}

pub fn create_launch_config(
    name: &str,
    program: &str,
    args: &[&str],
    working_dir: Option<&str>,
) -> Value {
    let mut config = Map::new();

    config.insert("name".to_string(), Value::String(name.to_string()));
    config.insert("type".to_string(), Value::String("node".to_string()));
    config.insert("request".to_string(), Value::String("launch".to_string()));
    config.insert("program".to_string(), Value::String(program.to_string()));

    if !args.is_empty() {
        let args_array: Vec<Value> = args
            .iter()
            .map(|arg| Value::String(arg.to_string()))
            .collect();
        config.insert("args".to_string(), Value::Array(args_array));
    }

    if let Some(dir) = working_dir {
        config.insert("cwd".to_string(), Value::String(dir.to_string()));
    } else {
        config.insert(
            "cwd".to_string(),
            Value::String("${workspaceFolder}".to_string()),
        );
    }

    config.insert(
        "console".to_string(),
        Value::String("integratedTerminal".to_string()),
    );
    config.insert(
        "internalConsoleOptions".to_string(),
        Value::String("neverOpen".to_string()),
    );

    Value::Object(config)
}

pub fn create_launch_json(configurations: Vec<Value>) -> Value {
    let mut launch = Map::new();
    launch.insert("version".to_string(), Value::String("0.2.0".to_string()));
    launch.insert("configurations".to_string(), Value::Array(configurations));

    Value::Object(launch)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_escape_string() {
        assert_eq!(json_escape_string("hello"), "hello");
        assert_eq!(json_escape_string("hello \"world\""), "hello \\\"world\\\"");
        assert_eq!(json_escape_string("path\\to\\file"), "path\\\\to\\\\file");
        assert_eq!(json_escape_string("line1\nline2"), "line1\\nline2");
        assert_eq!(json_escape_string("tab\there"), "tab\\there");
        assert_eq!(json_escape_string("return\rhere"), "return\\rhere");
    }

    #[test]
    fn test_create_vscode_task() {
        let task = create_vscode_task("Test Task", "echo", &["hello", "world"]);

        assert_eq!(
            extract_string_from_json(&task, "label"),
            Some("Test Task".to_string())
        );
        assert_eq!(
            extract_string_from_json(&task, "type"),
            Some("shell".to_string())
        );
        assert_eq!(
            extract_string_from_json(&task, "command"),
            Some("echo".to_string())
        );

        let args = extract_array_from_json(&task, "args").unwrap();
        assert_eq!(args.len(), 2);
        assert_eq!(args[0], Value::String("hello".to_string()));
        assert_eq!(args[1], Value::String("world".to_string()));
    }

    #[test]
    fn test_create_claude_task() {
        let task = create_claude_task("Test prompt", Some("/test/dir"));

        assert_eq!(
            extract_string_from_json(&task, "label"),
            Some("Para Session".to_string())
        );
        assert_eq!(
            extract_string_from_json(&task, "command"),
            Some("claude".to_string())
        );

        let args = extract_array_from_json(&task, "args").unwrap();
        assert!(args.len() >= 2);
        assert_eq!(args[0], Value::String("--prompt".to_string()));
        assert_eq!(args[1], Value::String("Test prompt".to_string()));
    }

    #[test]
    fn test_create_tasks_json() {
        let task1 = create_vscode_task("Task 1", "echo", &["test1"]);
        let task2 = create_vscode_task("Task 2", "echo", &["test2"]);

        let tasks_json = create_tasks_json(vec![task1, task2]);

        assert_eq!(
            extract_string_from_json(&tasks_json, "version"),
            Some("2.0.0".to_string())
        );

        let tasks = extract_array_from_json(&tasks_json, "tasks").unwrap();
        assert_eq!(tasks.len(), 2);
    }

    #[test]
    fn test_pretty_print_and_minify_json() {
        let mut obj = Map::new();
        obj.insert("key1".to_string(), Value::String("value1".to_string()));
        obj.insert("key2".to_string(), Value::Number(42.into()));
        let value = Value::Object(obj);

        let pretty = pretty_print_json(&value).unwrap();
        assert!(pretty.contains('\n'));
        assert!(pretty.contains("  "));

        let minified = minify_json(&value).unwrap();
        assert!(!minified.contains('\n'));
        assert!(!minified.contains("  "));
    }

    #[test]
    fn test_merge_json_objects() {
        let mut base = serde_json::json!({
            "existing": "value",
            "nested": {
                "keep": "this"
            }
        });

        let overlay = serde_json::json!({
            "new": "value",
            "nested": {
                "add": "this"
            }
        });

        merge_json_objects(&mut base, &overlay).unwrap();

        assert_eq!(
            extract_string_from_json(&base, "existing"),
            Some("value".to_string())
        );
        assert_eq!(
            extract_string_from_json(&base, "new"),
            Some("value".to_string())
        );

        let nested = extract_object_from_json(&base, "nested").unwrap();
        assert!(nested.contains_key("keep"));
        assert!(nested.contains_key("add"));
    }

    #[test]
    fn test_validate_json_structure() {
        let valid = serde_json::json!({
            "name": "test",
            "version": "1.0"
        });

        let invalid = serde_json::json!({
            "name": "test"
        });

        assert!(validate_json_structure(&valid, &["name", "version"]).is_ok());
        assert!(validate_json_structure(&invalid, &["name", "version"]).is_err());

        let not_object = serde_json::json!(["array"]);
        assert!(validate_json_structure(&not_object, &["name"]).is_err());
    }

    #[test]
    fn test_create_launch_config() {
        let config = create_launch_config(
            "Test Launch",
            "/path/to/program",
            &["arg1", "arg2"],
            Some("/working/dir"),
        );

        assert_eq!(
            extract_string_from_json(&config, "name"),
            Some("Test Launch".to_string())
        );
        assert_eq!(
            extract_string_from_json(&config, "program"),
            Some("/path/to/program".to_string())
        );
        assert_eq!(
            extract_string_from_json(&config, "cwd"),
            Some("/working/dir".to_string())
        );

        let args = extract_array_from_json(&config, "args").unwrap();
        assert_eq!(args.len(), 2);
    }

    #[test]
    fn test_create_launch_json() {
        let config1 = create_launch_config("Config 1", "/prog1", &[], None);
        let config2 = create_launch_config("Config 2", "/prog2", &[], None);

        let launch_json = create_launch_json(vec![config1, config2]);

        assert_eq!(
            extract_string_from_json(&launch_json, "version"),
            Some("0.2.0".to_string())
        );

        let configs = extract_array_from_json(&launch_json, "configurations").unwrap();
        assert_eq!(configs.len(), 2);
    }
}
