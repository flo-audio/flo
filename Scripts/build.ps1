#!/usr/bin/env pwsh
# Build script for libflo (PowerShell)
# Usage: .\scripts\build.ps1 [target]
# Targets: native, wasm, all (default)

param(
    [Parameter(Position=0)]
    [ValidateSet("native", "wasm", "all", "test", "clean", "help")]
    [string]$Command = "all"
)

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectDir = Split-Path -Parent $ScriptDir
$LibfloDir = Join-Path $ProjectDir "libflo"

function Write-Log {
    param([string]$Message)
    Write-Host "[build] " -ForegroundColor Blue -NoNewline
    Write-Host $Message
}

function Write-Success {
    param([string]$Message)
    Write-Host "[build] " -ForegroundColor Green -NoNewline
    Write-Host $Message
}

function Write-Error {
    param([string]$Message)
    Write-Host "[build] " -ForegroundColor Red -NoNewline
    Write-Host $Message
}

function Build-Native {
    Write-Log "Building native library..."
    Push-Location $LibfloDir
    try {
        cargo build --release
        if ($LASTEXITCODE -ne 0) { throw "Cargo build failed" }
        Write-Success "Native build complete: target/release/"
    }
    finally {
        Pop-Location
    }
}

function Build-Wasm {
    Write-Log "Building WASM library..."
    
    # Check for wasm-pack
    $wasmPack = Get-Command wasm-pack -ErrorAction SilentlyContinue
    if (-not $wasmPack) {
        Write-Log "Installing wasm-pack..."
        cargo install wasm-pack
    }
    
    Push-Location $LibfloDir
    try {
        wasm-pack build --release --target web --out-dir ../Demo/pkg-libflo
        if ($LASTEXITCODE -ne 0) { throw "wasm-pack build failed" }
        Write-Success "WASM build complete: Demo/pkg-libflo/"
    }
    finally {
        Pop-Location
    }
}

function Invoke-Tests {
    Write-Log "Running tests..."
    Push-Location $LibfloDir
    try {
        cargo test
        if ($LASTEXITCODE -ne 0) { throw "Tests failed" }
        Write-Success "All tests passed"
    }
    finally {
        Pop-Location
    }
}

function Invoke-Clean {
    Write-Log "Cleaning build artifacts..."
    Push-Location $LibfloDir
    try {
        cargo clean
        $PkgDir = Join-Path $ProjectDir "Demo/pkg"
        if (Test-Path $PkgDir) {
            Remove-Item -Recurse -Force $PkgDir
        }
        Write-Success "Clean complete"
    }
    finally {
        Pop-Location
    }
}

function Show-Help {
    Write-Host "flo™ Audio Codec Build Script"
    Write-Host ""
    Write-Host "Usage: .\build.ps1 [command]"
    Write-Host ""
    Write-Host "Commands:"
    Write-Host "  native    Build native Rust library"
    Write-Host "  wasm      Build WebAssembly package"
    Write-Host "  all       Build both native and WASM (default)"
    Write-Host "  test      Run all tests"
    Write-Host "  clean     Clean build artifacts"
    Write-Host "  help      Show this help"
}

# Main
switch ($Command) {
    "native" { Build-Native }
    "wasm" { Build-Wasm }
    "all" { Build-Native; Build-Wasm }
    "test" { Invoke-Tests }
    "clean" { Invoke-Clean }
    "help" { Show-Help }
}
