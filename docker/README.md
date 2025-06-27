# Docker Images for Para Development

This directory contains Docker configurations and scripts for para development with custom Docker images.

## Prerequisites

1. **Docker installed and running** (Docker Desktop, Colima, etc.)
2. **Para installed** (`just install` from the para repo)
3. **Para authenticated** (`para auth` to create `para-authenticated:latest`)

## The Custom Docker Workflow

Para supports repository-specific Docker images through `.para/Dockerfile.custom`. This allows each repository to define its own development environment.

### Step 1: Create Your Custom Dockerfile

Create `.para/Dockerfile.custom` in your repository:

```dockerfile
FROM para-claude:latest

# Install your project-specific tools
RUN sudo apt-get update && sudo apt-get install -y \
    build-essential \
    python3-pip \
    postgresql-client \
    && sudo rm -rf /var/lib/apt/lists/*

# Add any other customizations
WORKDIR /workspace
```

### Step 2: Build Your Custom Image

Run the build script:
```bash
./docker/build-custom-image.sh
```

This will:
- Look for `.para/Dockerfile.custom` first (repository-specific)
- Fall back to `docker/Dockerfile.para-dev` if not found
- Create an image named `para-<repo-name>:latest`

### Step 3: Create Authenticated Version

Add authentication to your custom image:
```bash
./docker/create-custom-authenticated.sh para-<repo-name>:latest
```

This creates an authenticated version by copying credentials from `para-authenticated:latest`.

### Step 4: Use Your Custom Image

```bash
para dispatch my-feature --container --docker-image para-authenticated:latest
```

The container now has:
- ✅ All tools from your custom Dockerfile
- ✅ Claude authentication
- ✅ Para integration

## Example: Para Development Image

The para repository includes its own `.para/Dockerfile.custom` with:
- Rust toolchain
- Just command runner
- Node.js/npm
- Build essentials
- Development tools (jq, ripgrep)

## Scripts

- `build-custom-image.sh`: Builds custom images from `.para/Dockerfile.custom`
- `create-custom-authenticated.sh`: Adds authentication to custom images
- `fresh-auth.sh`: Alternative script for fresh authentication (if needed)
- `reauth-custom-images.sh`: Reauthenticate when tokens expire

## File Structure

```
your-repo/
├── .para/
│   ├── Dockerfile.custom      # Your custom Docker configuration (tracked in git)
│   ├── worktrees/            # Para worktrees (ignored)
│   └── state/                # Para state (ignored)
└── docker/                   # Para docker scripts (from para repo)
    ├── build-custom-image.sh
    └── create-custom-authenticated.sh
```

## Notes

- The `.para/Dockerfile.custom` file is tracked in git (not ignored)
- Each repository can have its own custom development environment
- Authentication is copied from `para-authenticated:latest`, so you only authenticate once
- Custom images are layered on top of `para-claude:latest` for compatibility

## Reauthentication

When authentication tokens expire:

1. **Quick fix**: Run `./docker/reauth-custom-images.sh` to reauthenticate everything
2. **Manual**: Run `para auth reauth` then rebuild custom images
3. **Details**: See [REAUTHENTICATION.md](REAUTHENTICATION.md) for all options

