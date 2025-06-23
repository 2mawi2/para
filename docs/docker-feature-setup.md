# Docker Feature Setup Guide

## Enabling Docker Support

To enable Docker support in para, you need to:

### 1. Add Docker Feature to Cargo.toml

Add the following to your `Cargo.toml`:

```toml
[features]
default = []
docker = ["dep:bollard", "dep:async-trait"]

[dependencies]
# Existing dependencies...

# Docker dependencies (optional)
bollard = { version = "0.16", optional = true }
async-trait = { version = "0.1", optional = true }
```

### 2. Enable Docker Module

Uncomment the Docker module in `src/core/mod.rs`:

```rust
#[cfg(feature = "docker")]
pub mod docker;
```

### 3. Build with Docker Feature

```bash
# Build with Docker support
cargo build --features docker

# Run tests with Docker support
cargo test --features docker

# Install with Docker support
cargo install --path . --features docker
```

### 4. Configure Docker in para

Add Docker configuration to your `~/.config/para/config.json`:

```json
{
  "docker": {
    "enabled": true,
    "default_image": "ubuntu:22.04",
    "image_mappings": {
      "rust": "rust:latest",
      "node": "node:lts",
      "python": "python:3.11"
    }
  }
}
```

## Usage

Once enabled, you can use Docker with para:

```bash
# Start a session with Docker
para start my-feature --docker

# Execute commands in container
para docker exec -- cargo test

# Attach to container
para docker attach

# View container logs
para docker logs --follow
```

## Implementation Status

The Docker integration design includes:

- ✅ Module structure and types
- ✅ DockerService trait for abstraction
- ✅ DockerManager for coordination
- ✅ Configuration schema
- ✅ Status communication protocol
- ✅ Test infrastructure
- ⏳ DockerEngineService implementation (example provided)
- ⏳ CLI commands implementation
- ⏳ Integration with existing commands

The design is complete and ready for implementation. The actual Docker API integration would use the `bollard` crate as shown in the example implementation.