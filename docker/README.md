# Docker Images for Para Development

This directory contains Docker configurations for para development.

## Prerequisites

1. **Docker installed and running** (Docker Desktop, Colima, etc.)
2. **Para installed** (`just install` from the para repo)

## Workflow A: The Standard Para Workflow

This is the recommended workflow for most users. It relies on the official `para-claude:latest` image and the standard `para auth` command.

### Step 1: Ensure You Have the Base Image

Check if you have the `para-claude:latest` image:
```bash
docker images | grep para-claude
```

If you don't have it, you'll need to build or obtain the base image first. The `para-claude:latest` image should include the Claude Code CLI.

### Step 2: Authenticate

Run the standard authentication command:
```bash
para auth
```
This will create the `para-authenticated:latest` image, which is ready for use.

### Step 3: Use for Development

Now you can use the authenticated image for your development tasks:
```bash
# All agents now use your authenticated custom image
para dispatch my-feature --container

# para automatically uses para-authenticated:latest
```

## Customizing the Development Environment

If you need to add more tools to your development environment, you can build a custom image on top of `para-claude:latest`.

### Step 1: Modify the Dockerfile

Edit `Dockerfile.para-dev` to add your required tools. For example:

```dockerfile
FROM para-claude:latest

# Add your custom tools
RUN apt-get update && apt-get install -y \
    python3-pip \
    net-tools
```

### Step 2: Build Your Custom Dev Image

Run the build script:
```bash
./build-para-dev-image.sh
```
This creates a new `para-dev:latest` image with your tools.

### Step 3: Authenticate Your Custom Image

Now, run `para auth` and tell it to use your new dev image as the base:

```bash
para config set docker.base_image para-dev:latest
para auth
```

This will create `para-authenticated:latest` based on your customized `para-dev` image.

## Files

- `Dockerfile.para-dev`: Dockerfile for building a custom development image on top of `para-claude:latest`.
- `build-para-dev-image.sh`: Script to build the `para-dev:latest` image.
- `README.md`: This documentation file.

## Why This Workflow?

- **Simplicity**: Relies on standard `para` commands.
- **Consistency**: Ensures that the authentication mechanism is compatible with the base image.
- **Extensibility**: Allows for easy customization of the development environment without breaking core functionality.

