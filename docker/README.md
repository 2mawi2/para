# Docker Images for Para Development

This directory contains Docker configurations for para development.

## Quick Start

### 1. Build the Para Development Image
```bash
./build-para-dev-image.sh
```

This creates `para-dev:latest` with all dependencies pre-installed:
- Rust toolchain
- Just command runner  
- Node.js and npm/bun
- All system packages needed

### 2. Use for Development
```bash
# Start a new feature
para start my-feature --container --docker-image para-dev:latest

# The container starts instantly with everything ready!
# No waiting for dependency installation
```

## Files

- `Dockerfile.para-dev` - Dockerfile for the development image
- `build-para-dev-image.sh` - Script to build the image
- Other Docker files are for para's container features (not for para development itself)

## Benefits

1. **Fast Startup**: Containers start in seconds, not minutes
2. **Consistent Environment**: Same tools and versions every time
3. **Clean Isolation**: Each feature gets a fresh environment
4. **No Local Setup**: Don't need Rust/Node.js on your host

## Customization

Edit `Dockerfile.para-dev` to:
- Add more tools
- Pin specific Rust versions
- Include additional dependencies
- Set up your preferred shell/tools

Then rebuild with `./build-para-dev-image.sh`