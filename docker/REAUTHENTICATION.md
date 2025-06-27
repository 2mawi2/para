# Reauthentication Guide for Para Docker Containers

When Claude authentication tokens expire, you'll need to reauthenticate. This guide covers all scenarios.

## Signs Your Auth Has Expired

1. Claude commands fail inside containers with authentication errors
2. Agents report they can't authenticate with Claude
3. `claude --version` works but other commands fail with 401/403 errors

## Reauthentication Workflows

### Option 1: Standard Para Reauthentication (Recommended)

This is the simplest approach for standard para usage:

```bash
# Re-authenticate the base para image
para auth reauth

# This will:
# 1. Clean up old authentication
# 2. Create fresh para-authenticated:latest
```

**Note**: This only updates `para-authenticated:latest`. Custom images need to be rebuilt.

### Option 2: Rebuild Custom Authenticated Images

After reauthenticating the base image, rebuild your custom authenticated images:

```bash
# Step 1: Reauthenticate base image
para auth reauth

# Step 2: Rebuild your custom authenticated image
./docker/create-custom-authenticated.sh para-para:latest

# This copies the fresh auth from para-authenticated:latest
```

### Option 3: Fresh Authentication for Custom Images

If you prefer to authenticate each custom image directly:

```bash
# Authenticate a custom image from scratch
./docker/fresh-auth.sh para-para:latest

# This will:
# 1. Start a container from your custom image
# 2. Run 'claude /login' interactively
# 3. Save as para-para-authenticated:latest
```

### Option 4: In-Place Container Reauthentication (Advanced)

For a running container that needs reauthentication:

```bash
# 1. Enter the container
docker exec -it para-my-session bash

# 2. Re-authenticate inside
claude /login

# 3. Exit and commit the changes (optional)
docker commit para-my-session my-session-reauth:latest
```

## Automated Reauthentication Script

Create this helper script for quick reauthentication:

```bash
#!/bin/bash
# reauth-all.sh - Reauthenticate all para images

echo "🔐 Reauthenticating Para images..."

# Step 1: Reauthenticate base
echo "1️⃣ Reauthenticating base image..."
para auth reauth

# Step 2: Check for custom images
if [ -f .para/Dockerfile.custom ]; then
    echo "2️⃣ Found custom Dockerfile, rebuilding..."
    ./docker/build-custom-image.sh
    
    # Get the image name
    REPO_NAME=$(basename "$PWD")
    IMAGE_NAME="para-${REPO_NAME}:latest"
    
    echo "3️⃣ Creating authenticated version..."
    ./docker/create-custom-authenticated.sh "$IMAGE_NAME"
    
    echo "✅ Reauthentication complete!"
    echo "Use: para dispatch --container --docker-image para-authenticated:latest"
else
    echo "✅ Base reauthentication complete!"
fi
```

## Best Practices

1. **Regular Reauthentication**: Set a reminder to reauthenticate before tokens expire
2. **Test After Reauthentication**: Always test with a simple command like `claude --help`
3. **Keep Base Updated**: Always reauthenticate `para-authenticated:latest` first
4. **Document Token Lifetime**: Note when you authenticated to predict expiration

## Troubleshooting

### "Authentication failed" errors
- Check internet connectivity
- Ensure you're not behind a restrictive firewall
- Try `para auth cleanup` before `para auth setup`

### Custom image not picking up new auth
- Ensure `para-authenticated:latest` was reauthenticated first
- Rebuild with `create-custom-authenticated.sh`
- Check that the COPY commands in the script are correct

### Container can't access Claude after reauth
- The container might be using cached credentials
- Restart the container: `docker restart para-session-name`
- Or recreate: `para cancel session-name && para dispatch session-name ...`

## Token Expiration Timeline

While the exact expiration time varies, typical scenarios:
- Short-lived tokens: 1-7 days
- Standard tokens: 30-90 days
- Long-lived tokens: 6-12 months

Check your Claude authentication method to understand your token lifetime.