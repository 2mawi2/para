#!/bin/bash
# Quick script to view the Git Constellation visualization

echo "🌌 Opening Para Git Constellation..."
echo ""
echo "The visualization will open in your default browser."
echo "Make sure to:"
echo "  - Move your mouse over stars to see commit details"
echo "  - Click on stars for pulse effects"
echo "  - Watch for random shooting stars!"
echo ""

# Open the visualization
if command -v open &> /dev/null; then
    # macOS
    open index.html
elif command -v xdg-open &> /dev/null; then
    # Linux
    xdg-open index.html
elif command -v start &> /dev/null; then
    # Windows
    start index.html
else
    echo "Please open index.html in your web browser manually."
fi

echo "✨ Enjoy your journey through Para's cosmic history!"