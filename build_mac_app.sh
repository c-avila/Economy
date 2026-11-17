#!/usr/bin/env bash
# ==============================================================================
# DAX - Data Analysis Exchange (v0.2) — macOS Distributable .app & .zip Builder
# ==============================================================================
set -e

cd "$(dirname "$0")"

if ! command -v cargo >/dev/null 2>&1; then
    if [ -f "$HOME/.cargo/env" ]; then
        # shellcheck disable=SC1091
        source "$HOME/.cargo/env"
    else
        echo "[ERROR] Rust (cargo) not found."
        exit 1
    fi
fi

echo "[1/4] Building Rust release binary..."
cargo build --release

APP_NAME="DAX Economy"
BUNDLE_DIR="dist/${APP_NAME}.app"
CONTENTS_DIR="${BUNDLE_DIR}/Contents"
MACOS_DIR="${CONTENTS_DIR}/MacOS"
RESOURCES_DIR="${CONTENTS_DIR}/Resources"

echo "[2/4] Assembling ${BUNDLE_DIR}..."
rm -rf "${BUNDLE_DIR}"
mkdir -p "${MACOS_DIR}" "${RESOURCES_DIR}"

cp target/release/dax_economia_desktop "${MACOS_DIR}/dax_economia_desktop"
chmod +x "${MACOS_DIR}/dax_economia_desktop"

cp -R python_engine "${RESOURCES_DIR}/python_engine"
rm -rf "${RESOURCES_DIR}/python_engine/__pycache__"
cp requirements.txt "${RESOURCES_DIR}/requirements.txt"
if [ -f ".env.example" ]; then
    cp .env.example "${RESOURCES_DIR}/.env.example"
fi

cat > "${MACOS_DIR}/DAX_Economy_Launcher" << 'LAUNCHER_EOF'
#!/usr/bin/env bash
set -e

MACOS_DIR="$(cd "$(dirname "$0")" && pwd)"
CONTENTS_DIR="$(dirname "$MACOS_DIR")"
RESOURCES_DIR="${CONTENTS_DIR}/Resources"
APP_SUPPORT="$HOME/Library/Application Support/DAX_Economy"
VENV_DIR="${APP_SUPPORT}/.venv"

mkdir -p "${APP_SUPPORT}/data" "${APP_SUPPORT}/config"

if [ ! -x "${VENV_DIR}/bin/python3" ]; then
    PYTHON_BIN=""
    for candidate in python3 /opt/homebrew/bin/python3 /usr/local/bin/python3 /Library/Frameworks/Python.framework/Versions/Current/bin/python3; do
        if command -v "$candidate" >/dev/null 2>&1; then
            PYTHON_BIN="$(command -v "$candidate")"
            break
        fi
    done

    if [ -z "$PYTHON_BIN" ]; then
        osascript -e 'display dialog "Python 3 was not found on this Mac. Please install Python 3.10+ from python.org and reopen DAX Economy." buttons {"OK"} default button 1 with icon stop with title "DAX Economy"'
        exit 1
    fi

    osascript -e 'display notification "Setting up Python data engine for first use. Please wait..." with title "DAX Economy"' || true
    "$PYTHON_BIN" -m venv "$VENV_DIR"
    "${VENV_DIR}/bin/pip" install --quiet --upgrade pip
    "${VENV_DIR}/bin/pip" install --quiet -r "${RESOURCES_DIR}/requirements.txt"
fi

exec "${MACOS_DIR}/dax_economia_desktop"
LAUNCHER_EOF
chmod +x "${MACOS_DIR}/DAX_Economy_Launcher"

cat > "${CONTENTS_DIR}/Info.plist" << 'PLIST_EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>DAX Economy</string>
    <key>CFBundleDisplayName</key>
    <string>DAX Economy</string>
    <key>CFBundleIdentifier</key>
    <string>com.dax.economy</string>
    <key>CFBundleVersion</key>
    <string>0.2.0</string>
    <key>CFBundleShortVersionString</key>
    <string>0.2.0</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleExecutable</key>
    <string>DAX_Economy_Launcher</string>
    <key>LSMinimumSystemVersion</key>
    <string>12.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
PLIST_EOF

echo "[3/4] Creating distributable ZIP archive..."
rm -f "dist/DAX_Economy_macOS.zip"
(cd dist && zip -r -q "DAX_Economy_macOS.zip" "${APP_NAME}.app")

echo "[4/4] Done!"
echo "  App Bundle : dist/${APP_NAME}.app"
echo "  ZIP Package: dist/DAX_Economy_macOS.zip"
