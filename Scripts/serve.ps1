#!/usr/bin/env pwsh
# Development server for testing WASM demo (PowerShell)
# Usage: .\scripts\serve.ps1 [port]

param(
    [Parameter(Position=0)]
    [int]$Port = 8080
)

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectDir = Split-Path -Parent $ScriptDir
$DemoDir = Join-Path $ProjectDir "Demo"

Write-Host "[serve] " -ForegroundColor Blue -NoNewline
Write-Host "Starting development server..."
Write-Host "[serve] " -ForegroundColor Green -NoNewline
Write-Host "http://localhost:$Port"
Write-Host "[serve] " -ForegroundColor Blue -NoNewline
Write-Host "Press Ctrl+C to stop"
Write-Host ""

Push-Location $DemoDir
try {
    # Try Python first
    $python = Get-Command python3 -ErrorAction SilentlyContinue
    if (-not $python) {
        $python = Get-Command python -ErrorAction SilentlyContinue
    }
    
    if ($python) {
        & $python.Source -m http.server $Port
    }
    else {
        # Fallback to .NET HttpListener
        Write-Host "Python not found, using .NET HttpListener..."
        
        $listener = New-Object System.Net.HttpListener
        $listener.Prefixes.Add("http://localhost:$Port/")
        $listener.Start()
        
        while ($listener.IsListening) {
            $context = $listener.GetContext()
            $request = $context.Request
            $response = $context.Response
            
            $localPath = $request.Url.LocalPath
            if ($localPath -eq "/") { $localPath = "/index.html" }
            
            $filePath = Join-Path $DemoDir $localPath.TrimStart("/")
            
            if (Test-Path $filePath -PathType Leaf) {
                $content = [System.IO.File]::ReadAllBytes($filePath)
                
                # Set content type
                $ext = [System.IO.Path]::GetExtension($filePath).ToLower()
                $contentType = switch ($ext) {
                    ".html" { "text/html" }
                    ".js" { "application/javascript" }
                    ".css" { "text/css" }
                    ".wasm" { "application/wasm" }
                    ".json" { "application/json" }
                    default { "application/octet-stream" }
                }
                
                $response.ContentType = $contentType
                $response.ContentLength64 = $content.Length
                $response.OutputStream.Write($content, 0, $content.Length)
            }
            else {
                $response.StatusCode = 404
            }
            
            $response.Close()
        }
    }
}
finally {
    Pop-Location
}
