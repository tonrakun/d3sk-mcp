#Requires -Version 5.1
<#
.SYNOPSIS
    D3SK-MCP Installer for Windows
.DESCRIPTION
    Downloads the latest d3sk-mcp release from GitHub and installs it to
    %USERPROFILE%\.d3sk-mcp\, then writes an updater/launcher script there.
    Point Claude Desktop at the updater script to get auto-updates on every session.
#>

$ErrorActionPreference = "Stop"

$REPO    = "tonrakun/d3sk-mcp"
$INSTALL = Join-Path $env:USERPROFILE ".d3sk-mcp"
$ASSET   = "d3sk-mcp-windows-x86_64.zip"
$BINARY  = "d3sk-mcp.exe"

Write-Host "=== D3SK-MCP Installer ===" -ForegroundColor Cyan

# Fetch latest release metadata
Write-Host "Fetching latest release info..."
$release = Invoke-RestMethod -Uri "https://api.github.com/repos/$REPO/releases/latest" -UseBasicParsing
$version = $release.tag_name
Write-Host "Latest version: $version"

$asset = $release.assets | Where-Object { $_.name -eq $ASSET }
if (-not $asset) { throw "Asset '$ASSET' not found in release $version" }

# Create install directory
New-Item -ItemType Directory -Force $INSTALL | Out-Null

# Download archive
Write-Host "Downloading $ASSET..."
$tmp = Join-Path $env:TEMP "d3sk-mcp-install.zip"
Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $tmp -UseBasicParsing

# Extract binary
Write-Host "Extracting..."
Expand-Archive -Path $tmp -DestinationPath $INSTALL -Force
Remove-Item $tmp

# Write current version
[IO.File]::WriteAllText("$INSTALL\version.txt", $version)

# -------------------------------------------------------------------------
# Embed updater script
# -------------------------------------------------------------------------
$updater = @'
# D3SK-MCP Updater / Launcher
# Add this to Claude Desktop config:
#   "command": "powershell",
#   "args": ["-NoProfile", "-File", "<path to this file>"]
#
# On each MCP session start:
#   1. Spawns a background job that checks GitHub for a newer release.
#   2. If found, downloads the new binary as d3sk-mcp.pending.exe.
#   3. Launches the current binary (stdio pass-through for MCP protocol).
#   4. After the binary exits, applies the pending update if one arrived.

$INSTALL = $PSScriptRoot
$REPO    = "tonrakun/d3sk-mcp"
$BINARY  = Join-Path $INSTALL "d3sk-mcp.exe"
$PENDING = Join-Path $INSTALL "d3sk-mcp.pending.exe"
$VER     = Join-Path $INSTALL "version.txt"
$ASSET   = "d3sk-mcp-windows-x86_64.zip"

# Background update check — does not block MCP startup
$job = Start-Job -ScriptBlock {
    param($repo, $asset, $pending, $ver)
    try {
        $current = if (Test-Path $ver) { [IO.File]::ReadAllText($ver).Trim() } else { "" }

        $rel    = Invoke-RestMethod -Uri "https://api.github.com/repos/$repo/releases/latest" -UseBasicParsing
        $latest = $rel.tag_name
        if ($latest -eq $current) { return }

        $a = $rel.assets | Where-Object { $_.name -eq $asset }
        if (-not $a) { return }

        $tmp = [IO.Path]::GetTempFileName() + ".zip"
        Invoke-WebRequest -Uri $a.browser_download_url -OutFile $tmp -UseBasicParsing

        Add-Type -AssemblyName System.IO.Compression.FileSystem
        $zip   = [IO.Compression.ZipFile]::OpenRead($tmp)
        $entry = $zip.Entries | Where-Object { $_.Name -eq "d3sk-mcp.exe" }
        [IO.Compression.ZipFileExtensions]::ExtractToFile($entry, $pending, $true)
        $zip.Dispose()
        Remove-Item $tmp

        [IO.File]::WriteAllText($ver + ".pending", $latest)
    } catch {}
} -ArgumentList $REPO, $ASSET, $PENDING, $VER

# Launch MCP binary — stdin/stdout pass-through keeps MCP protocol intact
& $BINARY @args
$exitCode = $LASTEXITCODE

# Wait for update job (up to 60 s; usually finishes long before MCP exits)
Wait-Job $job -Timeout 60 | Out-Null
Remove-Job $job -Force -ErrorAction SilentlyContinue

# Apply pending update now that the old binary is no longer running
if (Test-Path $PENDING) {
    try {
        Copy-Item $PENDING $BINARY -Force
        Remove-Item $PENDING
        $pv = $VER + ".pending"
        if (Test-Path $pv) { Move-Item $pv $VER -Force }
    } catch {}
}

exit $exitCode
'@

[IO.File]::WriteAllText("$INSTALL\updater.ps1", $updater)

# -------------------------------------------------------------------------
# Summary
# -------------------------------------------------------------------------
$configPath = Join-Path $env:APPDATA "Claude\claude_desktop_config.json"
$escapedInstall = $INSTALL.Replace('\', '\\')

Write-Host ""
Write-Host "=== Installation complete ===" -ForegroundColor Green
Write-Host "Directory : $INSTALL"
Write-Host "Binary    : $INSTALL\$BINARY"
Write-Host "Updater   : $INSTALL\updater.ps1"
Write-Host "Version   : $version"
Write-Host ""
Write-Host "Add to Claude Desktop config ($configPath):" -ForegroundColor Yellow
Write-Host @"
{
  "mcpServers": {
    "d3sk-mcp": {
      "command": "powershell",
      "args": ["-NoProfile", "-File", "$escapedInstall\\updater.ps1"]
    }
  }
}
"@
