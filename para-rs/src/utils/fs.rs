use crate::utils::{ParaError, Result};
use directories::ProjectDirs;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

pub fn ensure_absolute_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        match env::current_dir() {
            Ok(current) => current.join(path),
            Err(_) => {
                // Fallback to an absolute path that works in tests
                if cfg!(test) {
                    std::env::temp_dir().join(path)
                } else {
                    PathBuf::from("/").join(path)
                }
            }
        }
    }
}

pub fn validate_directory_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(ParaError::invalid_args("Directory name cannot be empty"));
    }

    if name.contains('/') || name.contains('\\') {
        return Err(ParaError::invalid_args(
            "Directory name cannot contain path separators",
        ));
    }

    if name.starts_with('.') {
        return Err(ParaError::invalid_args(
            "Directory name cannot start with a dot",
        ));
    }

    if name.chars().any(|c| {
        c.is_control()
            || c == ':'
            || c == '*'
            || c == '?'
            || c == '"'
            || c == '<'
            || c == '>'
            || c == '|'
    }) {
        return Err(ParaError::invalid_args(
            "Directory name contains invalid characters",
        ));
    }

    Ok(())
}

pub fn create_dir_if_not_exists(path: &Path) -> Result<()> {
    if path.exists() {
        if !path.is_dir() {
            return Err(ParaError::file_operation(format!(
                "Path exists but is not a directory: {}",
                path.display()
            )));
        }
        return Ok(());
    }

    fs::create_dir_all(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            ParaError::permission_denied(path.display().to_string())
        } else {
            ParaError::file_operation(format!(
                "Failed to create directory {}: {}",
                path.display(),
                e
            ))
        }
    })
}

pub fn safe_remove_dir(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }

    if !path.is_dir() {
        return Err(ParaError::file_operation(format!(
            "Path is not a directory: {}",
            path.display()
        )));
    }

    fs::remove_dir_all(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            ParaError::permission_denied(path.display().to_string())
        } else {
            ParaError::file_operation(format!(
                "Failed to remove directory {}: {}",
                path.display(),
                e
            ))
        }
    })
}

pub fn read_file_content(path: &Path) -> Result<String> {
    if !path.exists() {
        return Err(ParaError::file_not_found(path.display().to_string()));
    }

    if !path.is_file() {
        return Err(ParaError::file_operation(format!(
            "Path is not a file: {}",
            path.display()
        )));
    }

    fs::read_to_string(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            ParaError::permission_denied(path.display().to_string())
        } else {
            ParaError::file_operation(format!("Failed to read file {}: {}", path.display(), e))
        }
    })
}

pub fn write_file_content(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        create_dir_if_not_exists(parent)?;
    }

    fs::write(path, content).map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            ParaError::permission_denied(path.display().to_string())
        } else {
            ParaError::file_operation(format!("Failed to write file {}: {}", path.display(), e))
        }
    })
}

pub fn copy_directory_contents(src: &Path, dst: &Path) -> Result<()> {
    if !src.exists() {
        return Err(ParaError::directory_not_found(src.display().to_string()));
    }

    if !src.is_dir() {
        return Err(ParaError::file_operation(format!(
            "Source is not a directory: {}",
            src.display()
        )));
    }

    create_dir_if_not_exists(dst)?;

    let entries = fs::read_dir(src).map_err(|e| {
        ParaError::file_operation(format!("Failed to read directory {}: {}", src.display(), e))
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| {
            ParaError::file_operation(format!("Failed to read directory entry: {}", e))
        })?;

        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_directory_contents(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path).map_err(|e| {
                ParaError::file_operation(format!(
                    "Failed to copy {} to {}: {}",
                    src_path.display(),
                    dst_path.display(),
                    e
                ))
            })?;
        }
    }

    Ok(())
}

pub fn is_file_path(input: &str) -> bool {
    let path = Path::new(input);

    if path.exists() {
        return path.is_file();
    }

    input.contains('/')
        || input.contains('\\')
        || input.ends_with(".txt")
        || input.ends_with(".md")
        || input.ends_with(".prompt")
}

pub fn find_git_repository() -> Result<PathBuf> {
    let mut current_dir = env::current_dir().map_err(|e| {
        ParaError::file_operation(format!("Failed to get current directory: {}", e))
    })?;

    loop {
        let git_dir = current_dir.join(".git");
        if git_dir.exists() {
            return Ok(current_dir);
        }

        match current_dir.parent() {
            Some(parent) => current_dir = parent.to_path_buf(),
            None => return Err(ParaError::repo_state("Not in a git repository")),
        }
    }
}

pub fn get_xdg_config_dir() -> Result<PathBuf> {
    if let Some(proj_dirs) = ProjectDirs::from("", "", "para") {
        Ok(proj_dirs.config_dir().to_path_buf())
    } else {
        let home = env::var("HOME")
            .or_else(|_| env::var("USERPROFILE"))
            .map_err(|_| ParaError::config_error("Unable to determine home directory"))?;
        Ok(PathBuf::from(home).join(".config").join("para"))
    }
}

pub fn get_para_config_path() -> Result<PathBuf> {
    let config_dir = get_xdg_config_dir()?;
    Ok(config_dir.join("config"))
}

pub fn get_para_state_dir() -> Result<PathBuf> {
    let repo_root = find_git_repository()?;
    Ok(repo_root.join(".para_state"))
}

pub fn ensure_file_exists(path: &Path) -> Result<()> {
    if !path.exists() {
        return Err(ParaError::file_not_found(path.display().to_string()));
    }
    Ok(())
}

pub fn ensure_dir_exists(path: &Path) -> Result<()> {
    if !path.exists() {
        return Err(ParaError::directory_not_found(path.display().to_string()));
    }
    if !path.is_dir() {
        return Err(ParaError::file_operation(format!(
            "Path exists but is not a directory: {}",
            path.display()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_ensure_absolute_path() {
        let relative = Path::new("test/path");
        let absolute = ensure_absolute_path(relative);
        assert!(absolute.is_absolute());

        let already_absolute = Path::new("/absolute/path");
        let result = ensure_absolute_path(already_absolute);
        assert_eq!(result, already_absolute);
    }

    #[test]
    fn test_validate_directory_name() {
        assert!(validate_directory_name("valid-name").is_ok());
        assert!(validate_directory_name("valid_name").is_ok());
        assert!(validate_directory_name("validname123").is_ok());

        assert!(validate_directory_name("").is_err());
        assert!(validate_directory_name("name/with/slash").is_err());
        assert!(validate_directory_name("name\\with\\backslash").is_err());
        assert!(validate_directory_name(".hidden").is_err());
        assert!(validate_directory_name("name:with:colon").is_err());
        assert!(validate_directory_name("name*with*asterisk").is_err());
    }

    #[test]
    fn test_create_and_remove_directory() {
        let temp_dir = TempDir::new().unwrap();
        let test_path = temp_dir.path().join("test_dir");

        assert!(create_dir_if_not_exists(&test_path).is_ok());
        assert!(test_path.exists());
        assert!(test_path.is_dir());

        assert!(create_dir_if_not_exists(&test_path).is_ok());

        assert!(safe_remove_dir(&test_path).is_ok());
        assert!(!test_path.exists());

        assert!(safe_remove_dir(&test_path).is_ok());
    }

    #[test]
    fn test_file_operations() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        let content = "test content";

        assert!(write_file_content(&test_file, content).is_ok());
        assert!(test_file.exists());

        let read_content = read_file_content(&test_file).unwrap();
        assert_eq!(read_content, content);

        let non_existent = temp_dir.path().join("non_existent.txt");
        assert!(read_file_content(&non_existent).is_err());
    }

    #[test]
    fn test_copy_directory_contents() {
        let temp_dir = TempDir::new().unwrap();
        let src_dir = temp_dir.path().join("src");
        let dst_dir = temp_dir.path().join("dst");

        fs::create_dir(&src_dir).unwrap();
        fs::write(src_dir.join("file1.txt"), "content1").unwrap();
        fs::write(src_dir.join("file2.txt"), "content2").unwrap();

        let sub_dir = src_dir.join("subdir");
        fs::create_dir(&sub_dir).unwrap();
        fs::write(sub_dir.join("file3.txt"), "content3").unwrap();

        assert!(copy_directory_contents(&src_dir, &dst_dir).is_ok());

        assert!(dst_dir.join("file1.txt").exists());
        assert!(dst_dir.join("file2.txt").exists());
        assert!(dst_dir.join("subdir").exists());
        assert!(dst_dir.join("subdir/file3.txt").exists());

        let content1 = fs::read_to_string(dst_dir.join("file1.txt")).unwrap();
        assert_eq!(content1, "content1");
    }

    #[test]
    fn test_is_file_path() {
        assert!(is_file_path("path/to/file.txt"));
        assert!(is_file_path("file.md"));
        assert!(is_file_path("prompt.prompt"));
        assert!(is_file_path("./relative/path"));
        assert!(is_file_path("../parent/path"));

        assert!(!is_file_path("simple-name"));
        assert!(!is_file_path("simple_name"));
        assert!(!is_file_path("123"));
    }

    #[test]
    fn test_get_xdg_config_dir() {
        let config_dir = get_xdg_config_dir().unwrap();
        assert!(config_dir.to_string_lossy().contains("para"));
    }
}
