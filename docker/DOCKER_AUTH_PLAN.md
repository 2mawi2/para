# Docker Authentication Plan for Para

## Current Situation

We have two incompatible approaches mixing together:
1. The original `para-claude:latest` with its own Claude installation
2. Our custom images with npm-installed Claude (`@anthropic-ai/claude-code`)

These two Claude installations store authentication differently and cannot share auth data.

## The Plan

### Phase 1: Clean Up Current Confusion
- [x] Understand that npm-installed Claude ≠ para-claude's Claude
- [x] Stop trying to mount auth volumes between different Claude installations
- [ ] Update documentation to reflect the correct workflow
- [ ] Remove the broken auth mounting code from `src/core/docker/service.rs`

### Phase 2: Implement Proper Workflows

#### Workflow A: Using para-claude Base (Simple Path)
1. **Revert Dockerfile.para-dev to use para-claude:latest as base**
   ```dockerfile
   FROM para-claude:latest
   # Add development tools on top
   ```

2. **Use standard para auth**
   ```bash
   para auth  # Creates para-authenticated:latest
   ```

3. **Build custom images from para-authenticated:latest**
   ```dockerfile
   FROM para-authenticated:latest
   # Add your tools
   ```

**Pros:** 
- Simple, uses existing auth mechanism
- No need for custom auth scripts

**Cons:** 
- Dependent on para-claude base image
- Less control over base environment

#### Workflow B: Custom Base with Fresh Auth (Full Control)
1. **Build from any base with npm-installed Claude**
   ```dockerfile
   FROM ubuntu:22.04  # Or any base
   RUN npm install -g @anthropic-ai/claude-code
   # Add your tools
   ```

2. **Fresh authentication per image**
   ```bash
   ./fresh-auth.sh my-custom-image:latest
   # This runs claude /login inside container
   # Saves authenticated state in the image
   ```

3. **Use authenticated custom image**
   ```bash
   para dispatch --container --docker-image my-custom-authenticated:latest
   ```

**Pros:**
- Full control over base image
- Can start from Alpine, Debian, Ubuntu, etc.
- Clear separation of concerns

**Cons:**
- Need to authenticate each custom image
- Authentication stored in image (not volume)

### Phase 3: Implementation Tasks

#### For Workflow A (para-claude base):
1. [ ] Revert Dockerfile.para-dev to use `FROM para-claude:latest`
2. [ ] Update build script to check for para-claude:latest
3. [ ] Update README with correct workflow
4. [ ] Test the workflow end-to-end

#### For Workflow B (custom base):
1. [ ] Keep Dockerfile.para-dev as is (FROM ubuntu:22.04)
2. [ ] Finalize fresh-auth.sh script
3. [ ] Remove auth volume mounting code from service.rs
4. [ ] Create clear documentation for fresh auth workflow
5. [ ] Add examples for different base images (Alpine, Debian)

### Phase 4: Long-term Improvements

1. **Enhance para auth to support custom base images**
   ```bash
   para auth --base-image my-custom:latest --output my-custom-authenticated:latest
   ```

2. **Create a para-base image with just Claude**
   - Minimal image with only Claude CLI
   - Users can build on top of this
   - Separates Claude from development tools

3. **Support for auth volumes with npm-installed Claude**
   - Research where npm Claude stores config
   - Create proper volume mounting strategy
   - Enable auth sharing between containers

## Recommendation

**Short term:** Use Workflow A (para-claude base) for immediate productivity
**Long term:** Implement proper support for Workflow B in para itself

## Decision Needed

Which workflow should we implement now?
1. **Workflow A**: Revert to para-claude base (simple, works today)
2. **Workflow B**: Fresh auth for custom images (more flexible, needs auth per image)
3. **Both**: Document both approaches, let users choose

## Next Steps

1. Make a decision on which workflow to pursue
2. Clean up the current mixed approach
3. Implement the chosen workflow cleanly
4. Update all documentation
5. Test end-to-end with real para agents