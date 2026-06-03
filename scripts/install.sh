#!/bin/sh
# D3SK-MCP Installer for macOS / Linux
#
# Downloads the latest d3sk-mcp release from GitHub and installs it to
# ~/.d3sk-mcp/, then writes an updater/launcher script there.
# Point Claude Desktop at the updater script to get auto-updates on every session.

set -e

REPO="tonrakun/d3sk-mcp"
INSTALL="$HOME/.d3sk-mcp"

echo "=== D3SK-MCP Installer ==="

# Detect platform
OS=$(uname -s)
ARCH=$(uname -m)
case "$OS-$ARCH" in
    Linux-x86_64)  ASSET="d3sk-mcp-linux-x86_64.tar.gz" ;;
    Darwin-x86_64) ASSET="d3sk-mcp-macos-x86_64.tar.gz" ;;
    Darwin-arm64)  ASSET="d3sk-mcp-macos-aarch64.tar.gz" ;;
    *) echo "Unsupported platform: $OS $ARCH" >&2; exit 1 ;;
esac

command -v curl >/dev/null 2>&1 || { echo "Error: curl is required" >&2; exit 1; }

# Fetch latest release metadata
echo "Fetching latest release info..."
RELEASE=$(curl -sf "https://api.github.com/repos/$REPO/releases/latest")
VERSION=$(echo "$RELEASE" | grep '"tag_name"' | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')
echo "Latest version: $VERSION"

DOWNLOAD_URL=$(echo "$RELEASE" | grep '"browser_download_url"' | grep "$ASSET" | sed 's/.*"browser_download_url": *"\([^"]*\)".*/\1/')
[ -n "$DOWNLOAD_URL" ] || { echo "Asset $ASSET not found in release $VERSION" >&2; exit 1; }

# Create install directory
mkdir -p "$INSTALL"

# Download and extract
echo "Downloading $ASSET..."
TMP=$(mktemp /tmp/d3sk-mcp-XXXXXX.tar.gz)
curl -sfL "$DOWNLOAD_URL" -o "$TMP"

echo "Extracting..."
tar xzf "$TMP" -C "$INSTALL"
rm -f "$TMP"
chmod +x "$INSTALL/d3sk-mcp"

# Write current version
printf '%s' "$VERSION" > "$INSTALL/version.txt"

# -------------------------------------------------------------------------
# Write updater / launcher script
# -------------------------------------------------------------------------
cat > "$INSTALL/updater.sh" << 'UPDATER_EOF'
#!/bin/sh
# D3SK-MCP Updater / Launcher
# Add this to Claude Desktop config:
#   "command": "/absolute/path/to/updater.sh"
#
# On each MCP session start:
#   1. Spawns a background subshell that checks GitHub for a newer release.
#   2. If found, downloads the new binary as d3sk-mcp.pending.
#   3. Launches the current binary (stdin/stdout pass-through for MCP protocol).
#   4. After the binary exits, applies the pending update if one arrived.

INSTALL="$(cd "$(dirname "$0")" && pwd)"
REPO="tonrakun/d3sk-mcp"
BINARY="$INSTALL/d3sk-mcp"
PENDING="$INSTALL/d3sk-mcp.pending"
VER="$INSTALL/version.txt"

# Detect platform asset name
OS=$(uname -s)
ARCH=$(uname -m)
case "$OS-$ARCH" in
    Linux-x86_64)  ASSET="d3sk-mcp-linux-x86_64.tar.gz" ;;
    Darwin-x86_64) ASSET="d3sk-mcp-macos-x86_64.tar.gz" ;;
    Darwin-arm64)  ASSET="d3sk-mcp-macos-aarch64.tar.gz" ;;
    *) ASSET="" ;;
esac

# Background update check — does not block MCP startup
UPDATE_PID=""
if [ -n "$ASSET" ] && command -v curl >/dev/null 2>&1; then
    (
        CURRENT=$(cat "$VER" 2>/dev/null || echo "")
        RELEASE=$(curl -sf "https://api.github.com/repos/$REPO/releases/latest" 2>/dev/null) || exit 0
        LATEST=$(echo "$RELEASE" | grep '"tag_name"' | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')
        [ -n "$LATEST" ] || exit 0
        [ "$LATEST" != "$CURRENT" ] || exit 0

        URL=$(echo "$RELEASE" | grep '"browser_download_url"' | grep "$ASSET" | sed 's/.*"browser_download_url": *"\([^"]*\)".*/\1/')
        [ -n "$URL" ] || exit 0

        TMPDIR=$(mktemp -d /tmp/d3sk-mcp-XXXXXX)
        TMPARCHIVE="$TMPDIR/archive.tar.gz"
        curl -sfL "$URL" -o "$TMPARCHIVE" || { rm -rf "$TMPDIR"; exit 0; }
        tar xzf "$TMPARCHIVE" -C "$TMPDIR" d3sk-mcp || { rm -rf "$TMPDIR"; exit 0; }
        mv "$TMPDIR/d3sk-mcp" "$PENDING"
        chmod +x "$PENDING"
        rm -rf "$TMPDIR"
        printf '%s' "$LATEST" > "$VER.pending"
    ) &
    UPDATE_PID=$!
fi

# Launch MCP binary — stdin/stdout pass-through keeps MCP protocol intact
"$BINARY" "$@"
EXIT_CODE=$?

# Wait for update job
if [ -n "$UPDATE_PID" ]; then
    wait "$UPDATE_PID" 2>/dev/null || true
fi

# Apply pending update now that the old binary is no longer running
if [ -f "$PENDING" ]; then
    mv -f "$PENDING" "$BINARY"
    chmod +x "$BINARY"
    if [ -f "$VER.pending" ]; then
        mv -f "$VER.pending" "$VER"
    fi
fi

exit $EXIT_CODE
UPDATER_EOF

chmod +x "$INSTALL/updater.sh"

# -------------------------------------------------------------------------
# Summary
# -------------------------------------------------------------------------
echo ""
echo "=== Installation complete ==="
echo "Directory : $INSTALL"
echo "Binary    : $INSTALL/d3sk-mcp"
echo "Updater   : $INSTALL/updater.sh"
echo "Version   : $VERSION"
echo ""
echo "Add to Claude Desktop config:"
cat << EOF
{
  "mcpServers": {
    "d3sk-mcp": {
      "command": "$INSTALL/updater.sh"
    }
  }
}
EOF
