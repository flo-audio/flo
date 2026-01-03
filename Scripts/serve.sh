#!/usr/bin/env bash
# Development server for testing WASM demo
# Usage: ./scripts/serve.sh [port]

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
DEMO_DIR="$PROJECT_DIR/Demo"
PORT="${1:-8080}"

# Colors
if [ -t 1 ]; then
    GREEN='\033[0;32m'
    BLUE='\033[0;34m'
    NC='\033[0m'
else
    GREEN=''
    BLUE=''
    NC=''
fi

echo -e "${BLUE}[serve]${NC} Starting development server..."
echo -e "${GREEN}[serve]${NC} http://localhost:$PORT"
echo -e "${BLUE}[serve]${NC} Press Ctrl+C to stop"
echo ""

cd "$DEMO_DIR"

# Try python3 first, then python
if command -v python3 &> /dev/null; then
    python3 -m http.server "$PORT"
elif command -v python &> /dev/null; then
    python -m http.server "$PORT"
else
    echo "Error: Python not found. Install Python or use another HTTP server."
    exit 1
fi
