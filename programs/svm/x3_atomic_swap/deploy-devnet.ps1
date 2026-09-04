# Deploy x3-atomic-swap Solana program to devnet (Windows)
param()

Write-Host "=== X3 Atomic Swap Solana Program Deploy (Devnet) ===" -ForegroundColor Cyan

# Check prerequisites
$hasSolana = Get-Command solana -ErrorAction SilentlyContinue
$hasCargoSbf = Get-Command cargo-build-sbf -ErrorAction SilentlyContinue
$hasCargoBpf = Get-Command cargo-build-bpf -ErrorAction SilentlyContinue

if (-not $hasSolana) {
    Write-Error "solana CLI not installed. Install from https://docs.solana.com/cli/install-solana-cli-tools"
    exit 1
}
if (-not ($hasCargoSbf -or $hasCargoBpf)) {
    Write-Error "cargo-build-sbf not installed. Run: cargo install solana-cli"
    exit 1
}

# Configure for devnet
Write-Host "Configuring solana CLI for devnet..."
solana config set --url https://api.devnet.solana.com

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path

# Build the BPF program
Write-Host "`nBuilding BPF program..."
if ($hasCargoSbf) {
    cargo build-sbf --manifest-path "$ScriptDir/Cargo.toml"
} else {
    cargo build-bpf --manifest-path "$ScriptDir/Cargo.toml"
}
if ($LASTEXITCODE -ne 0) {
    Write-Error "Build failed"
    exit 1
}

# Get the program keypair (create if not exists)
$ProgramDir = "$ScriptDir/target/deploy"
New-Item -ItemType Directory -Force -Path $ProgramDir | Out-Null
$ProgramKeypair = "$ProgramDir/x3_atomic_swap-keypair.json"

if (-not (Test-Path $ProgramKeypair)) {
    Write-Host "`nCreating program keypair..."
    solana-keygen new --no-bip39-passphrase -f -o "$ProgramKeypair"
}

$ProgramId = solana-keygen pubkey "$ProgramKeypair"
Write-Host "`nProgram ID: $ProgramId" -ForegroundColor Green

# Deploy
Write-Host "`nDeploying to Solana devnet..."
Write-Host "This may take a minute and cost SOL for rent + deployment fee.`n"
solana program deploy `
    --program-id "$ProgramKeypair" `
    "$ProgramDir/x3_atomic_swap.so"

if ($LASTEXITCODE -eq 0) {
    Write-Host "`n=== Deployment complete ===" -ForegroundColor Green
    Write-Host "Program ID: $ProgramId"
    Write-Host "Explorer: https://explorer.solana.com/address/$ProgramId?cluster=devnet"
    Write-Host "`nUpdate devnet-config.json and relayer config with this Program ID."
}
