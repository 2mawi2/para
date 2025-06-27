#!/bin/bash
set -e

# Secure entrypoint for Para Docker containers with network isolation
# This script sets up network security before running the main command

echo "🚀 Para Docker Container Starting..."
echo "Container ID: $(hostname)"
echo "Working directory: $(pwd)"

# Check if network isolation is enabled (default: false)
NETWORK_ISOLATION="${PARA_NETWORK_ISOLATION:-false}"

if [ "$NETWORK_ISOLATION" = "true" ]; then
    echo "🔒 Network isolation is enabled"
    
    # Check if we have the required capabilities using iptables test with sudo
    if ! sudo iptables -L >/dev/null 2>&1; then
        echo "❌ Error: Cannot access iptables. Network isolation requires NET_ADMIN and NET_RAW capabilities"
        echo "   Please ensure the container is running with: --cap-add=NET_ADMIN --cap-add=NET_RAW"
        exit 1
    fi
    
    # Check if iptables and ipset are available
    if ! command -v iptables >/dev/null 2>&1; then
        echo "❌ Error: iptables not found in container"
        echo "   Please rebuild the container image with iptables installed"
        exit 1
    fi
    
    if ! command -v ipset >/dev/null 2>&1; then
        echo "❌ Error: ipset not found in container"
        echo "   Please rebuild the container image with ipset installed"
        exit 1
    fi
    
    # Run the firewall initialization script with sudo
    echo "🔧 Configuring network firewall..."
    if [ -x /usr/local/bin/init-firewall.sh ]; then
        sudo /usr/local/bin/init-firewall.sh || {
            echo "❌ Failed to configure network isolation"
            echo "   Container will not start to prevent insecure operation"
            exit 1
        }
    else
        echo "❌ Error: Firewall script not found at /usr/local/bin/init-firewall.sh"
        exit 1
    fi
    
    echo "✅ Network isolation configured successfully"
else
    echo "⚠️  Network isolation is disabled"
    echo "   Container will have unrestricted network access"
fi

# Display container info
echo ""
echo "📋 Container Information:"
echo "   User: $(whoami)"
echo "   UID/GID: $(id)"
echo "   Working Dir: $(pwd)"
echo "   Network Isolation: $NETWORK_ISOLATION"

echo ""
echo "🎯 Ready to execute command: $*"
echo ""

# Execute the command directly
exec "$@"