# The Proper Docker Workflow for Para

## Understanding the Problem

When we install Claude via npm (`@anthropic-ai/claude-code`), it's a different installation than what's in `para-claude:latest`. Trying to share authentication between them is like trying to use Chrome's saved passwords in Firefox.

## Option 1: Use para-claude as Base (Recommended)

```dockerfile
FROM para-claude:latest

# Your tools on top
RUN apt-get update && apt-get install -y \
    build-essential \
    python3 \
    postgresql-client \
    # etc...
```

Then use normal `para auth` workflow.

## Option 2: Fresh Authentication for Custom Images

If you really want to build from scratch with npm-installed Claude:

### Step 1: Build Your Image
```bash
./build-para-dev-image.sh
```

### Step 2: Authenticate Fresh
```bash
./fresh-auth.sh para-dev:latest
```

This will:
1. Start a container from your image
2. Run `claude /login` inside it (you'll need to authenticate)
3. Save the authenticated state as `para-dev-authenticated:latest`

### Step 3: Use It
```bash
para dispatch my-feature --container --docker-image para-dev-authenticated:latest
```

## Why This Works

- Each image gets its own fresh authentication
- No trying to share auth data between different Claude installations
- Clean and predictable

## Important Notes

1. **You need to authenticate each custom image** - there's no magic auth sharing
2. **The auth is stored in the image** - not in a volume
3. **This is more secure** - each image has its own auth