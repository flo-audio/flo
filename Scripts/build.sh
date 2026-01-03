#!/usr/bin/env bash
# Build script for flo™
# Usage: ./scripts/build.sh [target]
# Targets: native, wasm, reflo, all (default)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
LIBFLO_DIR="$PROJECT_DIR/libflo"
REFLO_DIR="$PROJECT_DIR/reflo"

# Colors (if terminal supports it)
if [ -t 1 ]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    BLUE='\033[0;34m'
    BOLD='\033[1m'
    NC='\033[0m'
else
    RED=''
    GREEN=''
    BLUE=''
    BOLD=''
    NC=''
fi

log() {
    echo -e "${BLUE}[build]${NC} $1"
}

success() {
    echo -e "${GREEN}[build]${NC} $1"
}

error() {
    echo -e "${RED}[build]${NC} $1" >&2
}

build_native() {
    log "Building native library..."
    cd "$LIBFLO_DIR"
    cargo build --release
    success "Native build complete: target/release/liblibflo.*"
}

build_reflo() {
    log "Building reflo..."
    cd "$REFLO_DIR"
    cargo build --release
    success "reflo build complete: target/release/flo"
    echo ""
    log "Install with: cargo install --path reflo"
}

build_wasm() {
    log "Building WASM libraries..."
    
    # Check for wasm-pack
    if ! command -v wasm-pack &> /dev/null; then
        log "Installing wasm-pack..."
        cargo install wasm-pack
    fi
    
    # Build libflo WASM (for metadata functions)
    log "Building libflo WASM..."
    cd "$LIBFLO_DIR"
    wasm-pack build --release --target web
    cp package.json.template pkg/package.json
    mkdir -p ../Demo/pkg-libflo
    cp -r pkg/* ../Demo/pkg-libflo/
    success "libflo WASM build complete: Demo/pkg-libflo/"
    
    # Build reflo WASM (for encoding/decoding)
    log "Building reflo WASM..."
    cd "$REFLO_DIR"
    wasm-pack build --release --target web --features wasm
    cp package.json.template pkg/package.json
    mkdir -p ../Demo/pkg-reflo
    cp -r pkg/* ../Demo/pkg-reflo/
    success "reflo WASM build complete: Demo/pkg-reflo/"
}

run_tests() {
    log "Running tests..."
    cd "$LIBFLO_DIR"
    cargo test
    success "All tests passed"
}

show_help() {
    echo "flo™ Audio Codec Build Script"
    echo ""
    echo "Usage: $0 [command]"
    echo ""
    echo "Commands:"
    echo "  native            Build native Rust library"
    echo "  wasm              Build WebAssembly package"
    echo "  reflo             Build reflo tool"
    echo "  all               Build native, WASM, and reflo (default)"
    echo "  test              Run all tests"
    echo "  clean             Clean build artifacts"
    echo "  help              Show this help"
}

clean() {
    log "Cleaning build artifacts..."
    cd "$LIBFLO_DIR"
    cargo clean
    cd "$REFLO_DIR"
    cargo clean 2>/dev/null || true
    rm -rf "$PROJECT_DIR/Demo/pkg"
    success "Clean complete"
}

# Main
case "${1:-all}" in
    native)
        build_native
        ;;
    wasm)
        build_wasm
        ;;
    reflo)
        build_reflo
        ;;
    all)
        build_native
        build_reflo
        build_wasm
        ;;
    test)
        run_tests
        ;;
    clean)
        clean
        ;;
    help|--help|-h)
        show_help
        ;;
    *)
        error "Unknown command: $1"
        show_help
        exit 1
        ;;
esac
