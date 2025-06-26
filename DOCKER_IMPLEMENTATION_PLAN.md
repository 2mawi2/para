# Docker Feature Implementation Plan

## Phase 1: Environment Variable Forwarding (High Priority)

### 1.1 Modify DockerService to Forward Environment Variables

```rust
// In src/core/docker/service.rs

// Add these environment variables to forward by default
const DEFAULT_ENV_VARS: &[&str] = &[
    // API Keys
    "ANTHROPIC_API_KEY",
    "OPENAI_API_KEY", 
    "PERPLEXITY_API_KEY",
    "GOOGLE_API_KEY",
    "XAI_API_KEY",
    "OPENROUTER_API_KEY",
    "MISTRAL_API_KEY",
    "AZURE_OPENAI_API_KEY",
    "OLLAMA_API_KEY",
    
    // Terminal
    "TERM",
    "TERM_PROGRAM",
    "COLORTERM",
    
    // Proxy
    "HTTP_PROXY",
    "HTTPS_PROXY", 
    "NO_PROXY",
    "http_proxy",
    "https_proxy",
    "no_proxy",
    
    // Locale
    "LANG",
    "LC_ALL",
    
    // Editor
    "EDITOR",
    "VISUAL",
];

impl DockerService {
    pub fn create_container(
        &self,
        session_name: &str,
        network_isolation: bool,
        allowed_domains: &[String],
        working_dir: &Path,
        docker_args: &[String],
        forward_env_vars: &[String], // New parameter
    ) -> DockerResult<ContainerSession> {
        // ... existing code ...
        
        // Forward default environment variables
        for env_var in DEFAULT_ENV_VARS {
            if let Ok(value) = std::env::var(env_var) {
                docker_cmd_args.extend(["-e".to_string(), format!("{}={}", env_var, value)]);
            }
        }
        
        // Forward custom environment variables
        for env_var in forward_env_vars {
            if let Ok(value) = std::env::var(env_var) {
                docker_cmd_args.extend(["-e".to_string(), format!("{}={}", env_var, value)]);
            }
        }
        
        // ... rest of implementation ...
    }
}
```

### 1.2 Add CLI Support for Environment Variables

```rust
// In src/cli/parser.rs - Update DispatchArgs

#[derive(Args, Debug)]
pub struct DispatchArgs {
    // ... existing fields ...
    
    /// Forward specific environment variables to container (can be used multiple times)
    #[arg(long = "env", help = "Forward environment variable to container")]
    pub forward_env: Vec<String>,
    
    /// Forward all API key environment variables automatically
    #[arg(long, help = "Forward all API key environment variables")]
    pub forward_api_keys: bool,
}
```

## Phase 2: Custom Dockerfile Support

### 2.1 Add Dockerfile Detection and Building

```rust
// New file: src/core/docker/builder.rs

use super::{DockerError, DockerResult};
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct DockerImageBuilder;

impl DockerImageBuilder {
    /// Check if a custom Dockerfile exists in the project
    pub fn find_dockerfile(project_root: &Path) -> Option<PathBuf> {
        let candidates = [
            ".para/Dockerfile",
            ".para/dockerfile", 
            "para.Dockerfile",
            "Dockerfile.para",
        ];
        
        for candidate in &candidates {
            let path = project_root.join(candidate);
            if path.exists() {
                return Some(path);
            }
        }
        None
    }
    
    /// Build a custom Docker image from Dockerfile
    pub fn build_custom_image(
        dockerfile_path: &Path,
        image_name: &str,
        build_args: &[(String, String)],
    ) -> DockerResult<()> {
        let mut docker_cmd_args = vec![
            "build".to_string(),
            "-t".to_string(),
            image_name.to_string(),
            "-f".to_string(),
            dockerfile_path.to_string_lossy().to_string(),
        ];
        
        // Add build arguments
        for (key, value) in build_args {
            docker_cmd_args.extend([
                "--build-arg".to_string(),
                format!("{}={}", key, value),
            ]);
        }
        
        // Use project root as build context
        let build_context = dockerfile_path
            .parent()
            .and_then(|p| p.parent())
            .unwrap_or_else(|| Path::new("."));
            
        docker_cmd_args.push(build_context.to_string_lossy().to_string());
        
        println!("🔨 Building custom Docker image: {}", image_name);
        let output = Command::new("docker")
            .args(&docker_cmd_args)
            .output()
            .map_err(|e| DockerError::Other(anyhow::anyhow!("Failed to build image: {}", e)))?;
            
        if !output.status.success() {
            return Err(DockerError::Other(anyhow::anyhow!(
                "Failed to build Docker image: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        
        println!("✅ Successfully built image: {}", image_name);
        Ok(())
    }
}
```

### 2.2 Update DockerManager to Support Custom Images

```rust
// In src/core/docker/manager.rs

impl DockerManager {
    /// Get or build the appropriate Docker image
    fn get_or_build_image(&self, session_name: &str, force_build: bool) -> DockerResult<String> {
        let project_root = std::env::current_dir()
            .map_err(|e| DockerError::Other(anyhow::anyhow!("Failed to get current dir: {}", e)))?;
            
        // Check for custom Dockerfile
        if let Some(dockerfile_path) = DockerImageBuilder::find_dockerfile(&project_root) {
            let custom_image_name = format!("para-custom-{}:latest", session_name);
            
            // Check if image already exists (unless force build)
            if !force_build {
                let output = Command::new("docker")
                    .args(["images", "-q", &custom_image_name])
                    .output()
                    .map_err(|e| DockerError::DaemonNotAvailable(e.to_string()))?;
                    
                if output.status.success() && !output.stdout.is_empty() {
                    println!("📦 Using existing custom image: {}", custom_image_name);
                    return Ok(custom_image_name);
                }
            }
            
            // Build the custom image
            DockerImageBuilder::build_custom_image(
                &dockerfile_path,
                &custom_image_name,
                &[], // TODO: Add build args support
            )?;
            
            return Ok(custom_image_name);
        }
        
        // Fall back to default authenticated image
        self.get_docker_image().map(|s| s.to_string())
    }
}
```

## Phase 3: Docker Configuration Support

### 3.1 Add Docker Config Structure

```rust
// In src/config/mod.rs

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Config {
    pub ide: IdeConfig,
    pub directories: DirectoryConfig,
    pub git: GitConfig,
    pub session: SessionConfig,
    pub docker: DockerConfig, // New field
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct DockerConfig {
    /// Default Docker image to use
    pub default_image: Option<String>,
    
    /// Default environment variables to forward
    pub forward_env: Vec<String>,
    
    /// Additional volume mounts (source:target format)
    pub extra_mounts: Vec<String>,
    
    /// Default Docker arguments
    pub default_args: Vec<String>,
    
    /// Enable network isolation by default
    pub network_isolation: bool,
    
    /// Default allowed domains for network isolation
    pub allowed_domains: Vec<String>,
}

impl Default for DockerConfig {
    fn default() -> Self {
        Self {
            default_image: None,
            forward_env: vec![],
            extra_mounts: vec![],
            default_args: vec![],
            network_isolation: false,
            allowed_domains: vec![],
        }
    }
}
```

## Phase 4: Additional Volume Mounts

### 4.1 Extend DockerService for Multiple Mounts

```rust
// In src/core/docker/service.rs

impl DockerService {
    pub fn create_container(
        &self,
        session_name: &str,
        network_isolation: bool,
        allowed_domains: &[String],
        working_dir: &Path,
        docker_args: &[String],
        forward_env_vars: &[String],
        extra_mounts: &[(PathBuf, PathBuf)], // New parameter
    ) -> DockerResult<ContainerSession> {
        // ... existing code ...
        
        // Add extra volume mounts
        for (source, target) in extra_mounts {
            // Expand paths and validate
            let source_expanded = shellexpand::tilde(&source.to_string_lossy()).to_string();
            let source_path = Path::new(&source_expanded);
            
            if !source_path.exists() {
                eprintln!("Warning: Mount source does not exist: {}", source_path.display());
                continue;
            }
            
            docker_cmd_args.extend([
                "-v".to_string(),
                format!("{}:{}", source_path.display(), target.display()),
            ]);
        }
        
        // ... rest of implementation ...
    }
}
```

### 4.2 Add CLI Support for Mounts

```rust
// In src/cli/parser.rs - Update DispatchArgs

#[derive(Args, Debug)]
pub struct DispatchArgs {
    // ... existing fields ...
    
    /// Additional volume mounts (format: source:target)
    #[arg(long = "mount", help = "Additional volume mount (can be used multiple times)")]
    pub extra_mounts: Vec<String>,
}
```

## Implementation Order

1. **Week 1**: Environment Variable Forwarding
   - Update DockerService
   - Add CLI flags
   - Test with various API keys

2. **Week 2**: Custom Dockerfile Support  
   - Implement DockerImageBuilder
   - Update DockerManager
   - Test with sample Dockerfiles

3. **Week 3**: Configuration Support
   - Add DockerConfig to Config struct
   - Update config wizard
   - Migration for existing configs

4. **Week 4**: Volume Mounts
   - Extend DockerService
   - Add mount validation
   - Test with various mount scenarios

## Testing Plan

1. **Unit Tests**
   - Environment variable filtering
   - Dockerfile detection logic
   - Mount path validation

2. **Integration Tests**
   - Full container creation with env vars
   - Custom image building
   - Multiple mount points

3. **Manual Testing**
   - Test with real API keys
   - Build custom images with dependencies
   - Verify mount permissions