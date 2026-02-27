# DriveGuard Pro - Lambda Package Builder (PowerShell)
#
# Builds deployment zips for both Lambda functions and the shared
# firebase-admin Lambda Layer.
#
# Usage:
#   .\build_lambda_packages.ps1

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$OutputDir = Join-Path $ScriptDir "lambda_packages"
$LayerDir = Join-Path $ScriptDir "_build_layer"

Write-Host "================================================"
Write-Host " DriveGuard Pro - Lambda Package Builder"
Write-Host "================================================"
Write-Host ""

# Clean previous builds
if (Test-Path $LayerDir) { Remove-Item $LayerDir -Recurse -Force }
if (Test-Path $OutputDir) { Remove-Item $OutputDir -Recurse -Force }
New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null

# Step 1: Build firebase-admin Lambda Layer
Write-Host "[1/3] Building firebase-admin Lambda Layer..."
$pythonDir = Join-Path $LayerDir "python"
New-Item -ItemType Directory -Path $pythonDir -Force | Out-Null

pip install firebase-admin `
    --target $pythonDir `
    --platform manylinux2014_x86_64 `
    --implementation cp `
    --python-version 3.12 `
    --only-binary=:all: `
    --quiet

$layerZip = Join-Path $OutputDir "firebase_layer.zip"
Compress-Archive -Path (Join-Path $LayerDir "python") -DestinationPath $layerZip -Force
Remove-Item $LayerDir -Recurse -Force

$layerSize = (Get-Item $layerZip).Length / 1MB
Write-Host "   -> firebase_layer.zip ($([math]::Round($layerSize, 1)) MB)"

# Step 2: Package credential proxy Lambda
Write-Host "[2/3] Packaging credential proxy Lambda..."
$credProxyZip = Join-Path $OutputDir "credential_proxy.zip"
$credProxySrc = Join-Path (Join-Path $ScriptDir "aws_infrastructure") "lambda_credential_proxy.py"
Compress-Archive -Path $credProxySrc -DestinationPath $credProxyZip -Force
Write-Host "   -> credential_proxy.zip"

# Step 3: Package license manager Lambda
Write-Host "[3/3] Packaging license manager Lambda..."
$licMgrZip = Join-Path $OutputDir "license_manager.zip"
$licMgrSrc = Join-Path (Join-Path $ScriptDir "aws_infrastructure") "lambda_license_manager.py"
Compress-Archive -Path $licMgrSrc -DestinationPath $licMgrZip -Force
Write-Host "   -> license_manager.zip"

Write-Host ""
Write-Host "================================================"
Write-Host " Build complete! Packages in: lambda_packages/"
Write-Host "================================================"
Get-ChildItem $OutputDir | Format-Table Name, @{N="Size";E={"{0:N1} KB" -f ($_.Length/1KB)}} -AutoSize
Write-Host ""
Write-Host "Next steps:"
Write-Host "  1. cd terraform"
Write-Host "  2. terraform init"
Write-Host "  3. terraform apply -var='firebase_project_id=califax-g-488015' \"
Write-Host "       -var='firebase_credentials_file=C:\Users\bbaid\Downloads\califax-g-488015-e8e810ccc499.json' \"
Write-Host "       -var='bucket_name=YOUR-UNIQUE-BUCKET-NAME'"
Write-Host "================================================"
