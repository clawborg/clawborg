#!/usr/bin/env bash
set -e

echo "🤖 ClawBorg — Development Setup"
echo "================================"

# Check dependencies
command -v cargo >/dev/null 2>&1 || { echo "❌ Rust not found. Install: https://rustup.rs"; exit 1; }
command -v pnpm >/dev/null 2>&1 || { echo "❌ pnpm not found. Install: npm i -g pnpm"; exit 1; }

echo "✅ Rust $(rustc --version | cut -d' ' -f2)"
echo "✅ pnpm $(pnpm --version)"

# Frontend setup
echo ""
echo "📦 Installing frontend dependencies..."
cd web
pnpm install
pnpm build
cd ..

echo ""
echo "🔨 Building ClawBorg..."
cargo build --release

echo ""
echo "✅ Build complete!"
echo ""
echo "Run with mock data:"
echo "  ./target/release/clawborg --dir ./fixtures/mock-openclaw"
echo ""
echo "Run with real OpenClaw:"
echo "  ./target/release/clawborg"
echo ""
echo "Development mode (hot reload):"
echo "  Terminal 1: cargo run -- --dir ./fixtures/mock-openclaw"
echo "  Terminal 2: cd web && pnpm dev"
