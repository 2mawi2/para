# Troubleshooting

## 📋 Requirements

- **Git** with worktree support (Git 2.5+)
- **POSIX shell** (bash, zsh, fish, dash, ash, etc.)
- **Your preferred IDE** with CLI support (configure via `IDE_CMD`)

## 🐛 Common Issues

### "not in a Git repository"
**Problem:** Para reports that you're not in a Git repository  
**Solution:** Run para from within a Git repository directory

### "IDE CLI not found"
**Problem:** Para can't find your IDE command  
**Solutions:**
- Install your IDE's CLI or set `IDE_CMD` environment variable
- For Claude Code: ensure `claude` command is in PATH
- For Cursor: ensure `cursor` command is in PATH  
- For VS Code: ensure `code` command is in PATH

### "session not found"
**Problem:** Para can't find or auto-detect your session  
**Solutions:**
- Use `para list` to see active sessions
- Ensure you're in the correct directory for auto-detection
- Use `para resume <session-name>` to explicitly resume a session

### Changes Not Appearing
**Problem:** Your changes aren't showing up in the finished commit  
**Solutions:**
- Para auto-stages all changes during `finish` - no need to run `git add`
- Check that you're in the correct worktree directory
- Use `git status` to see what changes are detected

### Merge Conflicts
**Problem:** Getting conflicts when finishing sessions  
**Solution:** This is normal when multiple sessions modify the same files:
1. Edit the conflicted files to resolve conflicts
2. **Don't** run `git add` manually
3. Run `para continue` to auto-stage and complete the finish

### IDE Not Opening
**Problem:** Para creates session but IDE doesn't open  
**Solutions:**
- Verify IDE command works: `cursor --help`, `claude --help`, etc.
- Check IDE is properly installed with CLI support
- Try running the IDE command manually from the worktree directory

### Sessions Not Cleaning Up
**Problem:** Old sessions remain after `para clean`  
**Solutions:**
- Check for uncommitted changes that prevent cleanup
- Manually remove worktree directories in `subtrees/` if needed
- Remove state files in `.para_state/` if they become corrupted

### Performance Issues
**Problem:** Para feels slow  
**Solutions:**
- Large repositories may take longer for worktree operations
- Consider using a faster SSD if working with very large repos
- Use `para clean` regularly to remove old sessions

## 🔧 Advanced Troubleshooting

### Debug Mode
Run para with debug output to see what's happening:
```bash
set -x  # Enable debug mode
para start
set +x  # Disable debug mode
```

### Manual Cleanup
If para's automatic cleanup fails:
```bash
# Remove all worktrees manually
rm -rf subtrees/

# Remove state directory
rm -rf .para_state/

# Remove any remaining branches (be careful!)
git branch | grep "para/" | xargs git branch -D
```

### Check Git Worktree Status
See what Git worktrees exist:
```bash
git worktree list
```

Remove orphaned worktrees:
```bash
git worktree prune
``` 