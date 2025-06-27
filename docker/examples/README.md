# Example Docker Images for Para

This directory contains example Dockerfiles for common development stacks. All images are built on top of `para-claude:latest` to ensure Claude Code integration works properly.

## Prerequisites

Before building any of these images, ensure you have:
1. Run `para auth` to set up the base `para-claude:latest` image
2. Docker installed and running (Docker Desktop, Colima, etc.)

## Available Images

### Node.js Development
```bash
# Build the image
docker build -t para-node:latest -f Dockerfile.node .

# Use with para
para dispatch my-feature --container --docker-image para-node:latest
```

Includes:
- Node.js 20.x
- TypeScript, ts-node
- pnpm, yarn, nodemon
- Build essentials

### Python Development
```bash
# Build the image
docker build -t para-python:latest -f Dockerfile.python .

# Use with para
para dispatch my-feature --container --docker-image para-python:latest
```

Includes:
- Python 3.x with pip
- Common packages: numpy, pandas, requests
- Testing: pytest
- Linting: black, flake8, mypy
- Jupyter and IPython

### Rust Development
```bash
# Build the image
docker build -t para-rust:latest -f Dockerfile.rust .

# Use with para
para dispatch my-feature --container --docker-image para-rust:latest
```

Includes:
- Rust stable toolchain
- cargo-watch, cargo-edit, cargo-expand
- just command runner
- Build essentials and pkg-config

## Creating Your Own Image

To create a custom image for your project:

1. Start with `FROM para-claude:latest`
2. Switch to root user for installations: `USER root`
3. Install your dependencies
4. Set working directory: `WORKDIR /workspace`
5. End with: `CMD ["sleep", "infinity"]`

Example for a Go project:
```dockerfile
FROM para-claude:latest
USER root

RUN apt-get update && apt-get install -y golang-go && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /workspace
CMD ["sleep", "infinity"]
```

## Authentication

Para automatically mounts your Claude authentication when using custom images. You don't need to worry about authentication in your Dockerfile - just ensure you've run `para auth` once on your system.

## Tips

1. **Layer efficiently**: Put rarely-changing commands first
2. **Clean up**: Remove apt lists to reduce image size
3. **Use specific versions**: Pin tool versions for reproducibility
4. **Test locally**: Run `docker run --rm -it your-image bash` to test