# === M2TW Seamless Coop Packaging Script ===
$ErrorActionPreference = "Stop"

$workspaceRoot = "c:\Users\ENG AHMAD\Documents\Project_momo\M2TW_seamlessCoop"
$rustDir = "$workspaceRoot\rust_module"
$luaPluginDir = "C:\Program Files (x86)\Steam\steamapps\common\Medieval II Total War\mods\crusades\youneuoy_Data\plugins\lua"

# Output Directory
$releaseDir = "$workspaceRoot\Release_Package"
$modPacketDir = "$releaseDir\M2TW_Seamless_Coop"

Write-Host ">>> Starting Packaging Process..." -ForegroundColor Cyan

# 0. Clean previous build
if (Test-Path $releaseDir) {
    Write-Host "Cleaning old release folder..." -ForegroundColor Yellow
    Remove-Item -Recurse -Force $releaseDir
}
New-Item -ItemType Directory -Force -Path $modPacketDir | Out-Null

# 1. Build Rust Server (Release Mode)
Write-Host ">>> Building Rust Server (Release)..." -ForegroundColor Cyan
Push-Location $rustDir
cargo build --release
if ($LASTEXITCODE -ne 0) {
    Write-Error "Rust build failed!"
}
Pop-Location

# 2. Structure Folders
$serverDir = "$modPacketDir\Server"
$pluginsDir = "$modPacketDir\Plugins"
$luaDir = "$pluginsDir\lua"

New-Item -ItemType Directory -Force -Path $serverDir | Out-Null
New-Item -ItemType Directory -Force -Path $luaDir | Out-Null

# 3. Copy Rust Server & Config
Write-Host ">>> Copying Server Files..." -ForegroundColor Cyan
Copy-Item "$rustDir\target\release\rust_module.exe" "$serverDir\rust_server.exe"

# Create Template Config
$configContent = @"
# Medieval 2 Seamless Coop Server Configuration

# 1. Dropbox Access Token
# Generate a new token from the Dropbox Developer Console.
# IMPORTANT: This token must be kept secret.
dropbox_token = "INSERT_YOUR_DROPBOX_TOKEN_HERE"

# 2. Save Directory (Optional)
# If playing a mod other than Crusades, update this path.
# Default: C:\Program Files (x86)\Steam\steamapps\common\Medieval II Total War\mods\crusades\saves
# save_dir = "C:\\Program Files (x86)\\Steam\\steamapps\\common\\Medieval II Total War\\mods\\crusades\\saves"
"@
Set-Content -Path "$serverDir\config.toml" -Value $configContent

# Create Run Batch File
$batContent = @"
@echo off
cd /d "%~dp0"
echo Starting M2TW Seamless Coop Server...
rust_server.exe
pause
"@
Set-Content -Path "$serverDir\Run_Server.bat" -Value $batContent


# 4. Copy Lua Plugins & Redist
Write-Host ">>> Copying Lua Scripts & Redist..." -ForegroundColor Cyan

# Copy Main Lua Script
Copy-Item "$luaPluginDir\LuaPluginScript.lua" "$luaDir\"
Copy-Item "$luaPluginDir\myconfigs.lua" "$luaDir\"

# Copy Redist Folder (recursively)
# This includes the pipe_module (core.dll, init.lua) and other deps
Copy-Item -Recurse "$luaPluginDir\redist" "$luaDir\redist"

# 5. Create Install Instructions
Write-Host ">>> Creating Instructions..." -ForegroundColor Cyan
$readmeContent = @"
=== Medieval II Total War: Seamless Coop Mod ===
Installation Instructions

Prerequisites:
1. Medieval II Total War installed.
2. Engine Overhaul Project (EOP) installed for your mod (e.g., Crusades).

How to "Connect":
This mod uses Dropbox to sync save files between players. 
For this to work, YOU AND YOUR FRIEND MUST USE THE *SAME* DROPBOX ACCOUNT (or the same Access Token).
You are not connecting directly to each other; you are both connecting to the same Dropbox storage.

Installation:
1. Open your Mod folder (e.g., mods/crusades).
2. Go to 'youneuoy_Data/plugins'.
3. Copy the contents of the 'Plugins' folder from this package into 'youneuoy_Data/plugins'.
   - It should look like: youneuoy_Data/plugins/lua/LuaPluginScript.lua
   - And: youneuoy_Data/plugins/lua/redist/pipe_module/...

Server Setup:
1. Go to the 'Server' folder in this package.
2. Open 'config.toml' with Notepad.
3. Paste the SHARED Dropbox Access Token where it says "INSERT_YOUR_DROPBOX_TOKEN_HERE".
   - (For the Host: Generate this token from the Dropbox Console)
   - (For the Friend: Ask the Host to send you this token)
4. Run 'Run_Server.bat'.

Playing:
1. Start the game with EOP.
2. The Server window should be open.
3. When you end your turn, the mod will automatically upload the save to the shared Dropbox folder.
4. The other player will see the update and download it automatically.
"@
Set-Content -Path "$modPacketDir\ReadMe_Install.txt" -Value $readmeContent

# 6. Verify
Write-Host ">>> Packaging Complete!" -ForegroundColor Green
Write-Host "Package created at: $modPacketDir"
Get-ChildItem -Recurse $modPacketDir | Select-Object FullName
