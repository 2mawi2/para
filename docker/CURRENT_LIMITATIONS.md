# Current Docker Authentication Limitations

## The Issue
Currently, custom Docker images don't seamlessly inherit authentication from `para auth`. When you use a custom image like `para-dev:latest`, Claude may prompt for authentication on first use.

## Why This Happens
1. `para auth` creates an authenticated image (`para-authenticated:latest`) with credentials baked in
2. Custom images built from `para-claude:latest` don't have these credentials
3. Volume mounting for auth data has compatibility issues across different Claude versions

## Current Best Practice

### Option 1: Use Default Image + Install Tools
```bash
# Use the authenticated image
para dispatch my-feature --container

# Then install tools inside the container as needed
docker exec -it para-my-feature bash
apt-get update && apt-get install -y <your-tools>
```

### Option 2: Create Custom Authenticated Image
```bash
# Start from authenticated image
docker run -it --name temp-auth para-authenticated:latest bash

# Install your tools
apt-get update && apt-get install -y nodejs python3 rust

# Commit as new image
docker commit temp-auth my-authenticated:latest
docker rm temp-auth

# Use your custom authenticated image
para dispatch my-feature --container --docker-image my-authenticated:latest
```

## Future Improvement
The ideal workflow would be:
```bash
# Build custom image
./build-para-dev-image.sh

# Authenticate the custom image
para auth --base-image para-dev:latest --output para-dev-authenticated:latest

# Use authenticated custom image
para dispatch --container --docker-image para-dev-authenticated:latest
```

This would require enhancing `para auth` to support custom base images.