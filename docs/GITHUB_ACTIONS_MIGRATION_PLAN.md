# GitHub Actions Migration Plan for Para Container Workflows

## Executive Summary

This plan details migrating Para's Docker container workflows to GitHub Actions while preserving the current OAuth-based authentication mechanism. Users can authenticate once through browser login, and the authenticated container state gets reused across GitHub Actions runs.

## Current vs Proposed Architecture

### Current Local Docker Flow
```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   User runs     │    │   Para creates   │    │  Claude Code    │
│ para dispatch   │───▶│ Docker container │───▶│  runs inside    │
│                 │    │ with OAuth auth  │    │   container     │
└─────────────────┘    └──────────────────┘    └─────────────────┘
                                │
                                ▼
                       ┌──────────────────┐
                       │ para-authenticated:│
                       │     latest        │
                       │ (persisted image) │
                       └──────────────────┘
```

### Proposed GitHub Actions Flow
```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   User triggers │    │ GitHub Actions   │    │  Claude Code    │
│   workflow      │───▶│ pulls auth image │───▶│  runs inside    │
│   via UI/API    │    │ from registry    │    │ Actions runner  │
└─────────────────┘    └──────────────────┘    └─────────────────┘
                                │
                                ▼
                       ┌──────────────────┐
                       │Container Registry│
                       │(GitHub/DockerHub)│
                       │  Authenticated   │
                       │     Image        │
                       └──────────────────┘
```

## Authentication Flow Diagrams

### Phase 1: One-Time Authentication Setup

```
USER MACHINE                    GITHUB ACTIONS              CONTAINER REGISTRY
┌─────────────┐                ┌─────────────────┐         ┌─────────────────┐
│             │                │                 │         │                 │
│ 1. Run      │                │                 │         │                 │
│ para auth   │                │                 │         │                 │
│ setup-github│────────────────┤                 │         │                 │
│             │                │                 │         │                 │
│ 2. OAuth    │                │                 │         │                 │
│ Browser     │◄───────────────┤                 │         │                 │
│ Login       │                │                 │         │                 │
│             │                │                 │         │                 │
│ 3. Container│                │                 │         │                 │
│ Created     │                │                 │         │                 │
│ & Auth      │                │                 │         │                 │
│ Persisted   │                │                 │         │                 │
│             │                │                 │         │                 │
│ 4. Push     │                │                 │         │                 │
│ Auth Image  │────────────────┼─────────────────┼────────▶│ Store           │
│ to Registry │                │                 │         │ para-auth:user  │
│             │                │                 │         │                 │
└─────────────┘                └─────────────────┘         └─────────────────┘
```

### Phase 2: GitHub Actions Development Sessions

```
GITHUB ACTIONS RUNNER                    CONTAINER REGISTRY           PROJECT REPO
┌─────────────────────┐                 ┌─────────────────┐         ┌─────────────┐
│                     │                 │                 │         │             │
│ 1. Workflow         │                 │                 │         │             │
│ Triggered           │                 │                 │         │             │
│                     │                 │                 │         │             │
│ 2. Pull Auth        │────────────────▶│ para-auth:user  │         │             │
│ Container Image     │◄────────────────│                 │         │             │
│                     │                 │                 │         │             │
│ 3. Start Container  │                 │                 │         │             │
│ with Pre-Auth       │                 │                 │         │             │
│                     │                 │                 │         │             │
│ 4. Clone Project    │─────────────────┼─────────────────┼────────▶│ git clone   │
│                     │◄────────────────┼─────────────────┼─────────│             │
│                     │                 │                 │         │             │
│ 5. Claude Code      │                 │                 │         │             │
│ Runs (Pre-Auth)     │                 │                 │         │             │
│                     │                 │                 │         │             │
│ 6. Push Changes     │─────────────────┼─────────────────┼────────▶│ git push    │
│ & Create PR         │                 │                 │         │ create PR   │
│                     │                 │                 │         │             │
└─────────────────────┘                 └─────────────────┘         └─────────────┘
```

### Authentication State Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                    AUTHENTICATION LIFECYCLE                     │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────────────┐  │
│  │   STEP 1    │    │   STEP 2    │    │       STEP 3        │  │
│  │ User Setup  │───▶│ Auth Image  │───▶│ GitHub Actions Use  │  │
│  │ (One Time)  │    │ Creation    │    │ (Multiple Times)    │  │
│  └─────────────┘    └─────────────┘    └─────────────────────┘  │
│                                                                 │
│  Local Machine      Container Registry      GitHub Runners      │
│  ┌─────────────┐    ┌─────────────────┐    ┌─────────────────┐  │
│  │ para auth   │    │ para-auth:user  │    │ docker run      │  │
│  │ setup-github│───▶│ (authenticated) │───▶│ para-auth:user  │  │
│  │             │    │                 │    │ claude code ... │  │
│  └─────────────┘    └─────────────────┘    └─────────────────┘  │
│                                                                 │
│  OAuth Browser      Docker Push/Pull       Pre-Authenticated   │
│  Login Flow         Registry Sync          Claude Sessions     │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## Detailed Implementation

### 1. Authentication Setup Command

Add new command to para for GitHub Actions setup:

```bash
# New command: para auth setup-github
para auth setup-github [--registry docker.io] [--tag-prefix para-auth]
```

**Implementation in `lib/para-commands.sh`:**

```bash
cmd_auth() {
  subcommand="${1:-}"
  
  case "$subcommand" in
    "setup-github")
      shift
      setup_github_auth "$@"
      ;;
    *)
      echo "Usage: para auth setup-github [options]"
      echo "  --registry REGISTRY    Container registry (default: docker.io)"
      echo "  --tag-prefix PREFIX    Image tag prefix (default: para-auth)"
      exit 1
      ;;
  esac
}

setup_github_auth() {
  registry="docker.io"
  tag_prefix="para-auth"
  user_id=$(id -u -n)
  
  while [ $# -gt 0 ]; do
    case $1 in
      --registry)
        registry="$2"
        shift 2
        ;;
      --tag-prefix)
        tag_prefix="$2" 
        shift 2
        ;;
      *)
        echo "Unknown option: $1" >&2
        exit 1
        ;;
    esac
  done
  
  auth_image_name="${registry}/${tag_prefix}:${user_id}"
  
  echo "🔐 Setting up GitHub Actions authentication..."
  echo "📦 Auth image: $auth_image_name"
  
  # Check if already exists
  if docker manifest inspect "$auth_image_name" >/dev/null 2>&1; then
    echo "✅ Authenticated image already exists: $auth_image_name"
    echo "🔄 To re-authenticate, delete the image first:"
    echo "   docker rmi $auth_image_name"
    return 0
  fi
  
  # Create authentication container
  auth_container_name="para-auth-setup-$$"
  
  echo "🏗️  Creating authentication container..."
  
  # Build or ensure base image exists
  build_para_image
  
  # Create container for authentication
  docker run -d \
    --name "$auth_container_name" \
    --user para \
    -w /para-session \
    -v para-claude-license-$(id -u):/home/para/.claude \
    "${CONTAINER_IMAGE:-para-base:latest}" \
    sleep 3600
  
  echo "🌐 Starting Claude Code authentication..."
  echo "📝 A browser window will open for OAuth login"
  echo "⏳ Please complete the login process..."
  
  # Run Claude Code to trigger OAuth
  docker exec -it "$auth_container_name" claude auth login
  
  if [ $? -eq 0 ]; then
    echo "✅ Authentication successful!"
    echo "💾 Committing authenticated container to image..."
    
    # Commit the authenticated container
    docker commit "$auth_container_name" "$auth_image_name"
    
    echo "📤 Pushing authenticated image to registry..."
    docker push "$auth_image_name"
    
    echo "🎉 GitHub Actions setup complete!"
    echo ""
    echo "📋 Next steps:"
    echo "1. Add this image to your GitHub Actions workflow:"
    echo "   container: $auth_image_name"
    echo ""
    echo "2. Ensure your registry credentials are configured:"
    echo "   - For Docker Hub: DOCKER_USERNAME, DOCKER_PASSWORD secrets"
    echo "   - For GitHub: Already configured with GITHUB_TOKEN"
    echo ""
    echo "3. Use the provided workflow templates in:"
    echo "   .github/workflows/para-development.yml"
    
  else
    echo "❌ Authentication failed"
    docker rm -f "$auth_container_name" >/dev/null 2>&1
    return 1
  fi
  
  # Cleanup
  docker rm -f "$auth_container_name" >/dev/null 2>&1
}
```

### 2. GitHub Actions Workflow Templates

**`.github/workflows/para-development.yml`:**

```yaml
name: Para AI Development Session

on:
  workflow_dispatch:
    inputs:
      prompt:
        description: 'AI development task prompt'
        required: true
        type: string
      session_name:
        description: 'Session identifier (optional)'
        required: false
        type: string
      session_type:
        description: 'Number of parallel instances'
        required: true
        type: choice
        options:
          - single
          - multi-3
          - multi-5
        default: single
      auth_image:
        description: 'Authenticated container image'
        required: true
        type: string
        default: 'para-auth:${{ github.actor }}'

jobs:
  setup:
    runs-on: ubuntu-latest
    outputs:
      session_id: ${{ steps.session.outputs.session_id }}
      branch_name: ${{ steps.session.outputs.branch_name }}
      instance_count: ${{ steps.session.outputs.instance_count }}
    
    steps:
    - name: Generate Session Details
      id: session
      run: |
        SESSION_ID="${{ github.event.inputs.session_name || 'para' }}_$(date +%Y%m%d-%H%M%S)"
        BRANCH_NAME="para/$SESSION_ID"
        
        case "${{ github.event.inputs.session_type }}" in
          "multi-3") INSTANCE_COUNT=3 ;;
          "multi-5") INSTANCE_COUNT=5 ;;
          *) INSTANCE_COUNT=1 ;;
        esac
        
        echo "session_id=$SESSION_ID" >> $GITHUB_OUTPUT
        echo "branch_name=$BRANCH_NAME" >> $GITHUB_OUTPUT
        echo "instance_count=$INSTANCE_COUNT" >> $GITHUB_OUTPUT
        
        echo "🚀 Session: $SESSION_ID"
        echo "🌳 Branch: $BRANCH_NAME" 
        echo "🔢 Instances: $INSTANCE_COUNT"

  development:
    needs: setup
    runs-on: ubuntu-latest
    
    # Dynamic matrix based on instance count
    strategy:
      matrix:
        instance: ${{ fromJson(format('[{0}]', join(range(1, fromJson(needs.setup.outputs.instance_count) + 1), ','))) }}
      fail-fast: false
    
    # Use pre-authenticated container
    container:
      image: ${{ github.event.inputs.auth_image }}
      credentials:
        username: ${{ github.actor }}
        password: ${{ secrets.GITHUB_TOKEN }}
      options: --user para
    
    steps:
    - name: Checkout Repository
      uses: actions/checkout@v4
      with:
        fetch-depth: 0
        token: ${{ secrets.GITHUB_TOKEN }}
    
    - name: Configure Git
      run: |
        git config --global user.name "Para AI Assistant"
        git config --global user.email "para+${{ github.actor }}@ai.assistant"
        git config --global --add safe.directory $GITHUB_WORKSPACE
    
    - name: Create Feature Branch
      run: |
        BRANCH_NAME="${{ needs.setup.outputs.branch_name }}"
        if [ "${{ matrix.instance }}" != "1" ]; then
          BRANCH_NAME="${BRANCH_NAME}-${{ matrix.instance }}"
        fi
        
        echo "🌳 Creating branch: $BRANCH_NAME"
        git checkout -b "$BRANCH_NAME"
        
        echo "BRANCH_NAME=$BRANCH_NAME" >> $GITHUB_ENV
    
    - name: Verify Claude Code Authentication
      run: |
        echo "🔐 Verifying Claude Code authentication..."
        claude auth status || {
          echo "❌ Claude Code not authenticated in container"
          echo "💡 Please run: para auth setup-github"
          exit 1
        }
        echo "✅ Claude Code is authenticated"
    
    - name: Create Session Directory
      run: |
        SESSION_DIR="/para-session"
        mkdir -p "$SESSION_DIR"
        
        # Copy project files to session directory
        cp -r $GITHUB_WORKSPACE/* "$SESSION_DIR/" 2>/dev/null || true
        cd "$SESSION_DIR"
        
        echo "SESSION_DIR=$SESSION_DIR" >> $GITHUB_ENV
    
    - name: Run Claude Code Development Session
      working-directory: ${{ env.SESSION_DIR }}
      run: |
        echo "🤖 Starting Claude Code session..."
        echo "📝 Prompt: ${{ github.event.inputs.prompt }}"
        echo "🏷️  Instance: ${{ matrix.instance }}/${{ needs.setup.outputs.instance_count }}"
        
        # Run Claude Code with the provided prompt
        claude code "${{ github.event.inputs.prompt }}"
        
        echo "✅ Claude Code session completed"
    
    - name: Commit and Push Changes
      working-directory: ${{ env.SESSION_DIR }}
      run: |
        # Check if there are any changes
        if [ -z "$(git status --porcelain)" ]; then
          echo "ℹ️  No changes to commit"
          exit 0
        fi
        
        echo "📝 Committing changes..."
        git add .
        git status --short
        
        COMMIT_MSG="AI Implementation: ${{ github.event.inputs.prompt }}"
        if [ "${{ matrix.instance }}" != "1" ]; then
          COMMIT_MSG="$COMMIT_MSG (Instance ${{ matrix.instance }})"
        fi
        
        git commit -m "$COMMIT_MSG"
        
        echo "📤 Pushing to origin..."
        git push -u origin "$BRANCH_NAME"
        
        echo "✅ Changes pushed to branch: $BRANCH_NAME"
    
    - name: Save Session Artifacts
      uses: actions/upload-artifact@v4
      with:
        name: para-session-${{ needs.setup.outputs.session_id }}-${{ matrix.instance }}
        path: ${{ env.SESSION_DIR }}
        retention-days: 7
        if-no-files-found: warn

  create-pull-request:
    needs: [setup, development]
    runs-on: ubuntu-latest
    if: always() && needs.development.result != 'failure'
    
    steps:
    - name: Checkout Repository
      uses: actions/checkout@v4
      with:
        fetch-depth: 0
        token: ${{ secrets.GITHUB_TOKEN }}
    
    - name: Create Pull Request
      uses: peter-evans/create-pull-request@v5
      with:
        token: ${{ secrets.GITHUB_TOKEN }}
        title: '🤖 Para AI Development: ${{ github.event.inputs.prompt }}'
        body: |
          ## 🤖 AI-Generated Implementation
          
          **Prompt:** ${{ github.event.inputs.prompt }}
          **Session ID:** ${{ needs.setup.outputs.session_id }}
          **Session Type:** ${{ github.event.inputs.session_type }}
          **Instances Created:** ${{ needs.setup.outputs.instance_count }}
          
          ### 📋 Session Details
          - **Triggered by:** @${{ github.actor }}
          - **Workflow Run:** [${{ github.run_number }}](${{ github.server_url }}/${{ github.repository }}/actions/runs/${{ github.run_id }})
          - **Authentication Image:** `${{ github.event.inputs.auth_image }}`
          
          ### 🔗 Related Branches
          ${{ needs.setup.outputs.instance_count > 1 && format('- {0}-1\n- {0}-2\n', needs.setup.outputs.branch_name) || format('- {0}', needs.setup.outputs.branch_name) }}
          ${{ needs.setup.outputs.instance_count > 3 && format('- {0}-3\n', needs.setup.outputs.branch_name) || '' }}
          ${{ needs.setup.outputs.instance_count > 4 && format('- {0}-4\n- {0}-5\n', needs.setup.outputs.branch_name) || '' }}
          
          ---
          
          *This PR was automatically generated by Para's GitHub Actions workflow.*
        branch: ${{ needs.setup.outputs.branch_name }}
        delete-branch: false
        draft: ${{ needs.setup.outputs.instance_count > 1 }}  # Draft if multiple instances
```

### 3. File Input Support

**`.github/workflows/para-file-input.yml`:**

```yaml
name: Para File Input Development

on:
  workflow_dispatch:
    inputs:
      prompt_file:
        description: 'Path to prompt file in repository'
        required: true
        type: string
        default: 'prompts/development-task.md'
      auth_image:
        description: 'Authenticated container image'
        required: true
        type: string
        default: 'para-auth:${{ github.actor }}'

jobs:
  file-input-development:
    runs-on: ubuntu-latest
    
    container:
      image: ${{ github.event.inputs.auth_image }}
      credentials:
        username: ${{ github.actor }}
        password: ${{ secrets.GITHUB_TOKEN }}
      options: --user para
    
    steps:
    - name: Checkout Repository
      uses: actions/checkout@v4
      with:
        fetch-depth: 0
    
    - name: Validate Prompt File
      run: |
        PROMPT_FILE="${{ github.event.inputs.prompt_file }}"
        
        if [ ! -f "$PROMPT_FILE" ]; then
          echo "❌ Prompt file not found: $PROMPT_FILE"
          echo "📁 Available files in prompts/:"
          find . -name "*.md" -o -name "*.txt" -o -name "*.prompt" | head -10
          exit 1
        fi
        
        echo "✅ Prompt file found: $PROMPT_FILE"
        echo "📄 File size: $(wc -c < "$PROMPT_FILE") bytes"
        echo "📝 Preview:"
        head -5 "$PROMPT_FILE"
    
    - name: Execute File-Based Development
      run: |
        PROMPT_FILE="${{ github.event.inputs.prompt_file }}"
        SESSION_ID="file_$(basename "$PROMPT_FILE" | sed 's/\.[^.]*$//')_$(date +%Y%m%d-%H%M%S)"
        
        echo "🤖 Starting file-based Claude Code session..."
        echo "📂 Prompt file: $PROMPT_FILE"
        echo "🆔 Session ID: $SESSION_ID"
        
        # Read prompt from file and execute
        PROMPT_CONTENT=$(cat "$PROMPT_FILE")
        claude code "$PROMPT_CONTENT"
        
        echo "SESSION_ID=$SESSION_ID" >> $GITHUB_ENV
    
    - name: Commit and Create PR
      run: |
        git config --global user.name "Para AI Assistant"
        git config --global user.email "para+${{ github.actor }}@ai.assistant"
        git config --global --add safe.directory $GITHUB_WORKSPACE
        
        BRANCH_NAME="para/file-input-$SESSION_ID"
        git checkout -b "$BRANCH_NAME"
        
        if [ -n "$(git status --porcelain)" ]; then
          git add .
          git commit -m "File-based AI implementation: ${{ github.event.inputs.prompt_file }}"
          git push -u origin "$BRANCH_NAME"
          
          echo "✅ Changes pushed to branch: $BRANCH_NAME"
        else
          echo "ℹ️  No changes to commit"
        fi
```

### 4. Session Recovery Workflow

**`.github/workflows/para-recovery.yml`:**

```yaml
name: Para Session Recovery

on:
  workflow_dispatch:
    inputs:
      session_id:
        description: 'Session ID to recover (from artifacts)'
        required: true
        type: string
      instance:
        description: 'Instance number (for multi-instance sessions)'
        required: false
        type: string
        default: '1'
      auth_image:
        description: 'Authenticated container image'
        required: true
        type: string
        default: 'para-auth:${{ github.actor }}'

jobs:
  recover-session:
    runs-on: ubuntu-latest
    
    container:
      image: ${{ github.event.inputs.auth_image }}
      credentials:
        username: ${{ github.actor }}
        password: ${{ secrets.GITHUB_TOKEN }}
      options: --user para
    
    steps:
    - name: Download Session Artifacts
      uses: actions/download-artifact@v4
      with:
        name: para-session-${{ github.event.inputs.session_id }}-${{ github.event.inputs.instance }}
        path: ./recovered-session
    
    - name: Restore Session State
      run: |
        cd recovered-session
        
        echo "🔄 Recovering session: ${{ github.event.inputs.session_id }}"
        echo "📂 Session contents:"
        ls -la
        
        # Configure git
        git config --global user.name "Para AI Assistant"
        git config --global user.email "para+${{ github.actor }}@ai.assistant"
        git config --global --add safe.directory $(pwd)
        
        # Create recovery branch
        RECOVERY_BRANCH="para/recovered-${{ github.event.inputs.session_id }}-$(date +%H%M%S)"
        git checkout -b "$RECOVERY_BRANCH"
        
        echo "✅ Session recovered on branch: $RECOVERY_BRANCH"
        echo "🤖 Claude Code is ready for continued development"
        
        # Start interactive session
        claude code "Continue development from recovered session"
```

## Container Registry Options

### Option 1: GitHub Container Registry (Recommended)
```bash
# Setup for GitHub Container Registry
para auth setup-github --registry ghcr.io --tag-prefix para-auth

# Results in image: ghcr.io/para-auth:username
```

**Advantages:**
- ✅ Free for public repositories
- ✅ Integrated with GitHub authentication
- ✅ Automatic cleanup policies
- ✅ No additional credentials needed

### Option 2: Docker Hub
```bash
# Setup for Docker Hub
para auth setup-github --registry docker.io --tag-prefix username/para-auth

# Results in image: docker.io/username/para-auth:latest
```

**Advantages:**
- ✅ Widely supported
- ✅ Good free tier
- ⚠️ Requires DOCKER_USERNAME/DOCKER_PASSWORD secrets

### Option 3: Private Registry
```bash
# Setup for private registry
para auth setup-github --registry myregistry.com --tag-prefix para-auth

# Results in image: myregistry.com/para-auth:username
```

## Security Considerations

### Authentication Security Model

```
┌─────────────────────────────────────────────────────────────────┐
│                       SECURITY BOUNDARIES                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  LOCAL MACHINE          CONTAINER REGISTRY         GITHUB       │
│  ┌─────────────┐       ┌─────────────────┐       ┌───────────┐  │
│  │ OAuth Login │══════▶│ Encrypted Auth  │◄═════▶│ Secure    │  │
│  │ (One-time)  │       │ Container Image │       │ Runners   │  │
│  └─────────────┘       └─────────────────┘       └───────────┘  │
│                                                                 │
│  🔐 User Control       🛡️  Registry Security    🏛️  GitHub Infra│
│  - Interactive auth    - Image encryption       - Isolated VMs │
│  - Local credential    - Access controls        - No persistence│
│  - One-time setup      - Audit logging          - Clean runners │
│                                                                 │
├─────────────────────────────────────────────────────────────────┤
│                        THREAT MITIGATION                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ❌ NO API KEYS in GitHub Secrets                               │
│  ❌ NO persistent tokens on runners                             │
│  ❌ NO credential exposure in logs                              │
│  ✅ OAuth tokens encrypted in container                         │
│  ✅ Registry access controls                                    │
│  ✅ Ephemeral runner environments                               │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Best Practices

1. **Container Image Security:**
   ```bash
   # Use private registry for sensitive projects
   para auth setup-github --registry ghcr.io/private-org
   
   # Regular image rotation
   docker rmi ghcr.io/para-auth:$USER  # Forces re-authentication
   ```

2. **Access Controls:**
   ```yaml
   # Restrict workflow to specific users/teams
   on:
     workflow_dispatch:
   
   jobs:
     check-permissions:
       if: contains(fromJson('["authorized-user1", "authorized-user2"]'), github.actor)
   ```

3. **Audit Trail:**
   ```yaml
   # Log all para sessions
   - name: Log Session
     run: |
       echo "Para session by ${{ github.actor }}: ${{ github.event.inputs.prompt }}" >> audit.log
       git add audit.log
       git commit -m "Audit: Para session"
   ```

## Migration Checklist

### Phase 1: Setup (One-time per user)
- [ ] Install updated para with GitHub Actions support
- [ ] Run `para auth setup-github` for OAuth authentication
- [ ] Verify authenticated image in container registry
- [ ] Add workflow files to repository
- [ ] Test simple development session

### Phase 2: Workflow Integration
- [ ] Create prompt files for common tasks
- [ ] Configure repository secrets (if using private registry)
- [ ] Test multi-instance sessions
- [ ] Verify session recovery functionality
- [ ] Train team on new workflows

### Phase 3: Advanced Features
- [ ] Implement issue comment triggers (`@para implement X`)
- [ ] Add project management integration
- [ ] Set up automated testing of generated code
- [ ] Configure cost monitoring and alerts
- [ ] Create dashboard for session management

## Cost Estimation

### GitHub Actions Usage
```
Single Session (10 minutes):    $0.008  (Linux runner)
Multi-3 Session (30 minutes):   $0.024  (3 parallel runners)
Multi-5 Session (50 minutes):   $0.040  (5 parallel runners)

Monthly estimate (20 sessions): $0.16 - $0.80
```

### Storage Costs
```
Session Artifacts (7 days):     $0.008/GB/day
Container Registry:             Free (GitHub) / $0.50/GB/month (Docker Hub)
```

### Cost Optimization Tips
1. Use artifact cleanup policies
2. Limit session timeouts
3. Use conditional workflows for PR triggers
4. Implement session quotas per user

---

*This migration plan provides a complete path from local Docker containers to GitHub Actions while preserving the user-friendly OAuth authentication experience.*