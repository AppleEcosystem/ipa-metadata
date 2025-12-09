#!/bin/bash

# Ultra-optimized build script for maximum performance
# This script builds the IPA parser with all optimizations enabled

echo "🚀 Building IPA Parser with EXTREME optimizations..."
echo ""

# Set environment variables for maximum performance
export RUSTFLAGS="-C target-cpu=native -C opt-level=3 -C embed-bitcode=yes"
export CARGO_PROFILE_RELEASE_LTO=true
export CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1

# Clean previous builds
echo "🧹 Cleaning previous builds..."
cargo clean

# Build with release profile
echo "⚡ Compiling with native CPU optimizations..."
echo "   - Target CPU: native (using all available CPU features)"
echo "   - Optimization level: 3 (maximum)"
echo "   - LTO: fat (full link-time optimization)"
echo "   - Codegen units: 1 (maximum optimization)"
echo ""

RUSTFLAGS="-C target-cpu=native -C opt-level=3" cargo build --release

if [ $? -eq 0 ]; then
    echo ""
    echo "✅ Build successful!"
    echo ""
    echo "📦 Binary location: target/release/ipa-parser"
    
    # Show binary size
    if [ -f "target/release/ipa-parser" ]; then
        SIZE=$(du -h target/release/ipa-parser | cut -f1)
        echo "📏 Binary size: $SIZE"
    fi
    
    echo ""
    echo "🎯 Run with: ./target/release/ipa-parser --help"
    echo ""
    echo "⚡ Performance tips:"
    echo "   - This binary is optimized for YOUR specific CPU"
    echo "   - Use --multiple for parallel processing"
    echo "   - Use --no-icons for metadata-only (faster)"
    echo ""
else
    echo ""
    echo "❌ Build failed!"
    exit 1
fi
