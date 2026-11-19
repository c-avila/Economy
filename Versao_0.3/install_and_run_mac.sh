#!/usr/bin/env bash
# ==============================================================================
# DAX - Data Analysis Exchange (v0.2) — macOS Setup & Launch Script
# ==============================================================================
set -e

cd "$(dirname "$0")"

echo "============================================================"
echo "  DAX - Data Analysis Exchange (v0.2) — macOS Launcher"
echo "============================================================"

if ! command -v python3 >/dev/null 2>&1; then
    echo "[ERROR] python3 not found. Please install Python 3.10+ from https://www.python.org/downloads/"
    exit 1
fi

if ! command -v cargo >/dev/null 2>&1; then
    if [ -f "$HOME/.cargo/env" ]; then
        # shellcheck disable=SC1091
        source "$HOME/.cargo/env"
    else
        echo "[ERROR] Rust (cargo) not found. Install it with:"
        echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        exit 1
    fi
fi

if [ ! -d ".venv" ]; then
    echo "[1/3] Creating Python virtual environment (.venv)..."
    python3 -m venv .venv
fi

echo "[2/3] Installing/verifying Python dependencies..."
.venv/bin/pip install --quiet --upgrade pip
.venv/bin/pip install --quiet -r requirements.txt

echo "[3/3] Building and launching DAX Economy (Release mode)..."
cargo run --release
