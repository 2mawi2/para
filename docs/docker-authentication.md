# Docker Claude Authentication

This document describes how Para handles Claude authentication for Docker containers.

## Overview

Para uses pre-authenticated Docker images that have Claude credentials baked in during the image build process. This approach ensures consistent authentication across all containers without requiring runtime credential passing.

## Authentication Setup

### Building the Authenticated Image

Before using Docker containers with Para, you need to build the `para-authenticated:latest` image with your Claude credentials already configured inside it:

```bash
# Build the authenticated image using the provided Dockerfile
# Note: This assumes you have already authenticated Claude on your host system
docker build -f docker/Dockerfile.authenticated -t para-authenticated:latest .
```

### Using Authenticated Containers

Once the authenticated image is built, Para will automatically use it for all container sessions:

```bash
# Start a new session with Docker container
para start my-feature --container

# Start an AI-assisted session with container
para start "implement feature X" --container
```

## How It Works

1. **Pre-baked Authentication**: Claude credentials are included in the Docker image during build time
2. **No Runtime Setup**: Containers start with authentication already configured
3. **Consistent State**: All containers use the same authenticated base image

## Security Considerations

- Credentials are stored within the Docker image layers
- The authenticated image is local to your machine
- Each user must build their own authenticated image
- No credentials are passed via environment variables or command line

## Platform Support

- **macOS**: Full support
- **Linux**: Full support
- **Windows**: Not yet supported

## Troubleshooting

If containers fail to authenticate:

1. **Check Docker status**: Ensure Docker is running
   ```bash
   docker version
   ```

2. **Verify authenticated image exists**:
   ```bash
   docker images | grep para-authenticated
   ```

3. **Rebuild the authenticated image** if needed:
   ```bash
   docker build -f docker/Dockerfile.authenticated -t para-authenticated:latest .
   ```

## Technical Details

The pre-authenticated image approach:
- Eliminates runtime credential management complexity
- Provides consistent authentication across all containers
- Avoids credential extraction from host systems
- Simplifies container creation and startup