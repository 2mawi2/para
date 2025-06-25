#\!/bin/bash
echo "Testing status file location fix..."

# Stop daemon
./target/debug/para daemon stop 2>/dev/null || true
sleep 1

# Start daemon with debug binary
echo "Starting daemon with debug binary..."
./target/debug/para daemon start

# Check daemon status
./target/debug/para daemon status

echo -e "\nDaemon started with the fixed code."
echo "Now when you update status in the container, it should save to:"
echo "  /Users/marius.wichtner/Documents/git/para/.para/state/"
echo "instead of:"
echo "  $(pwd)/.para/state/"
