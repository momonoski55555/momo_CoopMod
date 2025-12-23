# === M2TW Lua Script Sync ===
$ErrorActionPreference = "Stop"

Write-Host ">>> Starting Lua Script Sync..." -ForegroundColor Cyan
Write-Host ""

# Find workspace root (current script directory)
$workspaceRoot = Split-Path -Parent $PSScriptRoot
if (-not $workspaceRoot) {
    $workspaceRoot = Get-Location
}
Write-Host "Workspace Root: $workspaceRoot" -ForegroundColor Gray

# Find Medieval II Total War installation
Write-Host "Searching for Medieval II Total War installation..." -ForegroundColor Yellow

$possiblePaths = @(
    "C:\Program Files (x86)\Steam\steamapps\common\Medieval II Total War",
    "C:\Program Files\Steam\steamapps\common\Medieval II Total War",
    "D:\SteamLibrary\steamapps\common\Medieval II Total War",
    "E:\SteamLibrary\steamapps\common\Medieval II Total War"
)

# Try to find Steam library folders from registry
try {
    $steamPath = (Get-ItemProperty -Path "HKLM:\SOFTWARE\WOW6432Node\Valve\Steam" -ErrorAction SilentlyContinue).InstallPath
    if ($steamPath) {
        $possiblePaths += "$steamPath\steamapps\common\Medieval II Total War"
        
        # Check libraryfolders.vdf for additional Steam libraries
        $libraryFile = "$steamPath\steamapps\libraryfolders.vdf"
        if (Test-Path $libraryFile) {
            $libraryContent = Get-Content $libraryFile -Raw
            $libraryContent | Select-String '"path"\s+"([^"]+)"' -AllMatches | ForEach-Object {
                $_.Matches | ForEach-Object {
                    $libPath = $_.Groups[1].Value -replace '\\\\', '\'
                    $possiblePaths += "$libPath\steamapps\common\Medieval II Total War"
                }
            }
        }
    }
} catch {
    Write-Host "Could not read Steam registry, using default paths..." -ForegroundColor Gray
}

# Find the actual installation
$m2twRoot = $null
foreach ($path in $possiblePaths) {
    if (Test-Path $path) {
        $m2twRoot = $path
        Write-Host "Found M2TW at: $m2twRoot" -ForegroundColor Green
        break
    }
}

if (-not $m2twRoot) {
    Write-Error "Could not find Medieval II Total War installation. Please install the game or specify the path manually."
    exit 1
}

# Find lua plugin directory
$luaPluginDir = $null
$modDirs = Get-ChildItem "$m2twRoot\mods" -Directory -ErrorAction SilentlyContinue

foreach ($modDir in $modDirs) {
    $testPath = "$($modDir.FullName)\youneuoy_Data\plugins\lua"
    if (Test-Path $testPath) {
        $luaPluginDir = $testPath
        Write-Host "Found Lua plugins in mod: $($modDir.Name)" -ForegroundColor Green
        break
    }
}

if (-not $luaPluginDir) {
    Write-Error "Could not find youneuoy_Data\plugins\lua directory in any mod folder. Is EOP installed?"
    exit 1
}

Write-Host "Lua Plugin Dir: $luaPluginDir" -ForegroundColor Gray
Write-Host ""

# Target directory in workspace
$targetLuaDir = "$workspaceRoot\M2TW_seamlessCoop\Lua"

# Create target directory if it doesn't exist
if (-not (Test-Path $targetLuaDir)) {
    Write-Host "Creating Lua directory at: $targetLuaDir" -ForegroundColor Yellow
    New-Item -ItemType Directory -Force -Path $targetLuaDir | Out-Null
}

# Define files to sync
$filesToSync = @(
    "LuaPluginScript.lua",
    "init.lua"
)

# Sync each file
foreach ($file in $filesToSync) {
    $sourcePath = Join-Path $luaPluginDir $file
    $targetPath = Join-Path $targetLuaDir $file
    
    if (Test-Path $sourcePath) {
        Write-Host "Syncing: $file" -ForegroundColor Green
        Copy-Item -Path $sourcePath -Destination $targetPath -Force
        Write-Host "  -> Copied to $targetPath" -ForegroundColor Gray
    } else {
        Write-Host "Warning: $file not found at $sourcePath" -ForegroundColor Yellow
    }
}

Write-Host ""
Write-Host ">>> Sync Complete!" -ForegroundColor Cyan
Write-Host "Files synced to: $targetLuaDir" -ForegroundColor Green

# List synced files
Write-Host ""
Write-Host "Synced files:" -ForegroundColor Cyan
Get-ChildItem $targetLuaDir | Select-Object Name, Length, LastWriteTime | Format-Table -AutoSize