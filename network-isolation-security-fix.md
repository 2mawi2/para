# Critical Security Fix: Network Isolation Image Validation

## Security Issue
If users enable network isolation but the Docker image doesn't have the secure entrypoint, the container will start WITHOUT isolation while showing it's enabled. This is a critical security vulnerability.

## Required Fixes

### 1. Image Validation Before Container Creation

In `src/core/docker/service.rs`, add validation before creating container:

```rust
pub fn create_container(
    &self,
    session_name: &str,
    config: &DockerConfig,
    working_dir: &Path,
    docker_args: &[String],
) -> DockerResult<ContainerSession> {
    // CRITICAL: Validate image has secure entrypoint if network isolation is enabled
    if config.network_isolation {
        self.validate_secure_image()?;
    }
    
    // ... rest of create_container
}

fn validate_secure_image(&self) -> DockerResult<()> {
    // Check if image has the secure entrypoint
    let output = Command::new("docker")
        .args(["inspect", "para-authenticated:latest", "--format", "{{json .Config.Entrypoint}}"])
        .output()
        .map_err(|e| DockerError::DaemonNotAvailable(e.to_string()))?;
    
    if !output.status.success() {
        return Err(DockerError::ImageNotFound(
            "para-authenticated:latest image not found. Please build with: para auth setup".to_string()
        ));
    }
    
    let entrypoint = String::from_utf8_lossy(&output.stdout);
    if !entrypoint.contains("secure-entrypoint.sh") {
        return Err(DockerError::InsecureImage(
            "SECURITY ERROR: Docker image does not support network isolation. Please rebuild with: para auth setup --force".to_string()
        ));
    }
    
    Ok(())
}
```

### 2. Runtime Verification After Container Start

Add a post-start check to verify isolation is actually working:

```rust
pub fn start_container(&self, session_name: &str) -> DockerResult<()> {
    // ... existing start logic ...
    
    // Verify network isolation if enabled
    if self.is_network_isolation_enabled(session_name)? {
        self.verify_network_isolation(session_name)?;
    }
    
    Ok(())
}

fn verify_network_isolation(&self, session_name: &str) -> DockerResult<()> {
    let container_name = format!("para-{}", session_name);
    
    // Check if iptables rules are applied
    let output = Command::new("docker")
        .args(["exec", &container_name, "iptables", "-L", "-n"])
        .output();
    
    match output {
        Ok(result) if result.status.success() => {
            let rules = String::from_utf8_lossy(&result.stdout);
            if !rules.contains("PARA_ALLOWED") {
                return Err(DockerError::NetworkIsolationFailed(
                    "SECURITY ERROR: Network isolation enabled but firewall rules not applied. Stopping container.".to_string()
                ));
            }
            Ok(())
        }
        _ => {
            // If we can't verify, fail safe
            return Err(DockerError::NetworkIsolationFailed(
                "SECURITY ERROR: Cannot verify network isolation. Stopping container for safety.".to_string()
            ));
        }
    }
}
```

### 3. Add New Error Types

In `src/core/docker/error.rs`:

```rust
#[derive(Debug, thiserror::Error)]
pub enum DockerError {
    // ... existing errors ...
    
    #[error("Insecure image: {0}")]
    InsecureImage(String),
    
    #[error("Network isolation verification failed: {0}")]
    NetworkIsolationFailed(String),
}
```

### 4. Image Rebuild Detection

Add image metadata to track when it needs rebuilding:

```rust
fn check_image_version(&self) -> DockerResult<bool> {
    // Check image labels for version/features
    let output = Command::new("docker")
        .args(["inspect", "para-authenticated:latest", "--format", "{{json .Config.Labels}}"])
        .output()
        .map_err(|e| DockerError::DaemonNotAvailable(e.to_string()))?;
    
    if !output.status.success() {
        return Ok(false); // Image doesn't exist
    }
    
    let labels = String::from_utf8_lossy(&output.stdout);
    Ok(labels.contains("\"para.network-isolation\":\"true\""))
}
```

### 5. Update Dockerfile to Include Labels

In `docker/Dockerfile.claude`:

```dockerfile
# Add labels for feature detection
LABEL para.version="1.2.0" \
      para.network-isolation="true" \
      para.secure-entrypoint="true"
```

### 6. Automatic Image Rebuild Prompt

When network isolation is requested but image is outdated:

```rust
if config.network_isolation && !self.check_image_version()? {
    return Err(DockerError::InsecureImage(
        "Docker image is outdated and doesn't support network isolation.\n\
         Please rebuild with: para auth setup --force\n\
         This is required for security features to work correctly.".to_string()
    ));
}
```

## Testing Checklist

1. [ ] Old image + network isolation requested = Error before container creation
2. [ ] New image + network isolation enabled = Verified after start
3. [ ] Runtime verification failure = Container stopped immediately
4. [ ] Clear error messages guide users to rebuild image

## Security Principles

1. **Fail Closed**: If we can't verify security, don't start
2. **Defense in Depth**: Multiple checks at different stages
3. **Clear Communication**: Users must understand the security state
4. **No Silent Failures**: Every security check must be explicit