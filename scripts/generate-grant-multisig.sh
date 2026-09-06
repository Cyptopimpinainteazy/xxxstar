# scripts/generate-grant-multisig.sh
#
# X3 Atomic Star — Grant-treasury multisig generation procedure.
#
# This script is DOCUMENTATION, not a generator. It prints the EXACT shell
# commands you need to run on an OFFLINE / AIR-GAPPED MACHINE in order to
# produce the multisig addresses for grant deposits. Running these on this
# online machine would leak key material into this workspace's session log.
#
# Prerequisites:
#   - One air-gapped machine (no network) — old laptop, Tails from USB, etc.
#   - `subkey` v3+ installed (https://github.com/paritytech/subkey)
#   - Two hardware wallets (Ledger / Trezor) — init'd, firmware updated
#   - Pen, paper, steel seed plates (Cryptosteel / Billfodl)
#   - 30 minutes of uninterrupted time
#
# What this script produces (when you run it offline):
#   - Three signer suris (12/24-word mnemonics) per network
#   - Multi-signature SS58 addresses for the grant deposit
#   - Steel seed plate content to record by hand
#   - Verification commands to confirm correctness
#
# Networks covered:
#   - Polkadot / Asset Hub / parachains  (Substrate ss58 address)
#   - Ethereum / L2s (Safe via safe.global UI)
#   - Solana     (Squads via Squads UI)
#
# Run order:
#   1. ./generate-grant-multisig.sh polkadot       (3 signers + 1 multisig)
#   2. ./generate-grant-multisig.sh evm             (Safe instructions)
#   3. ./generate-grant-multisig.sh solana         (Squads instructions)
#
# Acceptance: write NOTHING in this repo. The output addresses are for your
# password manager only. Verify with the readback steps at the end before
# sharing any address with a grant program.

set -euo pipefail

usage() {
  cat <<EOF
X3 Grant Treasury Multisig Generation Helper

USAGE:
  ./generate-grant-multisig.sh polkadot
  ./generate-grant-multisig.sh evm
  ./generate-grant-multisig.sh solana

This script prints the exact commands to run on an air-gapped machine.
Do NOT execute the printed commands on the machine holding this repo.

Required reading first:
  - TREASURY_POLICY.md (sections 2 and 3)
  - docs/SECRET_MANAGEMENT_POLICY.md
EOF
}

if [ $# -lt 1 ]; then
  usage
  exit 1
fi

NETWORK="$1"

case "$NETWORK" in
  polkadot)
    cat <<'EOF'

==========================================================================
POLKADOT GRANT MULTISIG (for DOT, parachain tokens, asset-hub assets)
==========================================================================

STEP 1 — On the AIR-GAPPED machine, install subkey:

  # Download subkey release binary from https://github.com/paritytech/subkey/releases
  # Verify the SHA256 hash on the github page
  # chmod +x subkey && mv subkey /usr/local/bin/

STEP 2 — Generate 3 signer suris (one per signer):

  signer1_mnemonic=$(subkey generate --scheme sr25519 --network substrate --output-type=json | jq -r .secretSeed)
  echo "SIGNER 1 (founder, hardware wallet 1):"
  echo "$signer1_mnemonic"

  signer2_mnemonic=$(subkey generate --scheme sr25519 --network substrate --output-type=json | jq -r .secretSeed)
  echo "SIGNER 2 (co-signer 1, hardware wallet 2):"
  echo "$signer2_mnemonic"

  signer3_mnemonic=$(subkey generate --scheme sr25519 --network substrate --output-type=json | jq -r .secretSeed)
  echo "SIGNER 3 (co-signer 2, hardware wallet 3):"
  echo "$signer3_mnemonic"

STEP 3 — Derive SSR55 addresses from each signer (for multisig setup UI):

  signer1_addr=$(subkey inspect --scheme sr25519 --network substrate "$signer1_mnemonic" | grep SS58 | awk '{print $2}')
  signer2_addr=$(subkey inspect --scheme sr25519 --network substrate "$signer2_mnemonic" | grep SS58 | awk '{print $2}')
  signer3_addr=$(subkey inspect --scheme sr25519 --network substrate "$signer3_mnemonic" | grep SS58 | awk '{print $2}')
  echo "Signer addresses:"
  echo "  founder:       $signer1_addr"
  echo "  co-signer 1:   $signer2_addr"
  echo "  co-signer 2:   $signer3_addr"

STEP 4 — Go ONLINE with these addresses only. Open
  https://polkadot.js.org/apps/?rpc=wss%3A%2F%2Frpc.polkadot.io#/multisig
  Or via Asset Hub for stable tokens:
  https://polkadot.js.org/apps/?rpc=wss%3A%2F%2Fkusama-asset-hub-rpc.polkadot.io#/multisig
  (replace RPC with the network you want the multisig on)

  - Click "create multisig account"
  - Signers: paste the three addresses from Step 3
  - Threshold: 2
  - Name:   "X3-Grant-Treasury"

  The UI shows you the resulting multisig address. WRITE IT DOWN:
  x3-grant-polkadot: <SS58 address here, eg 13UVJ...>

STEP 5 — RECORD the 3 mnemonics on steel seed plates BEFORE powering off
         the air-gapped machine. Two copies minimum, two physical locations
         (home safe, bank deposit box).

STEP 6 — VERIFY: on a different (non-air-gapped but TRUSTED) machine, with
  only the public addresses and one of the mnemonics known, derive the
  signer address and confirm it matches the Step-3 output. If it matches,
  the generation procedure is sound.

WHAT TO DO NEXT:
  1. Store the 3 mnemonics on hardware wallets (Ledger / Trezor).
  2. Initialize each hardware wallet with its mnemonic; record the device's
     BIP39 passphrase (passphrase+25th word) ONLY if you choose to use one
     (and back that up separately, also on steel).
  3. Add the multisig address to your password manager entry
     "X3 Treasury > Polkadot".
  4. Do NOT paste the mnemonics anywhere — not in this repo, not in a
     private git repo, not in a Notion page, nowhere digital outside the
     hardware wallet.

EOF
    ;;

  evm)
    cat <<'EOF'

==========================================================================
EVM GRANT SAFE (for USDC, ETH, stables on Ethereum / Base / any EVM)
==========================================================================

For EVM grant deposits we use Safe (formerly Gnosis Safe). It has
hardware-wallet-native signing via the Safe UI. This is the cleanest
multisig setup for EVM and uses battle-tested audit + 5+ years of
mainnet operation.

STEP 1 — Hardware wallets setup (one-time):

  For each signer:
  - Get a hardware wallet (Ledger Nano X / Stax, or Trezor Safe 5)
    — buy DIRECTLY from manufacturer (third-party markets can be tampered)
  - Initialize device, set PIN, record 24-word recovery seed on steel
  - Install EVM app on Ledger via Ledger Live (Ethereum app pre-installed
    on Stax; for Nano X install "Ethereum" app)
  - On Trezor: install "Ethereum" via Trezor Suite
  - Generate an Ethereum receive address on the hardware wallet
    (this will be signer address for Safe)
  - Record each signer address here:
    signer1 (founder, ledger 1): 0x....        Safe signer
    signer2 (co-signer 1, ledger 2): 0x....    Safe signer
    signer3 (co-signer 2, ledger 3): 0x....    Safe signer

STEP 2 — On a TRUSTED computer (NOT this repo machine):

  Open https://safe.global  (or https://app.safe.global)
  Connect each of the 3 hardware wallets via browser extension / WalletConnect
  Click "Create new Safe"
  - Network:  Choose your target (Ethereum mainnet for USDC, or Base, etc.)
  - Name:     "X3-Grant-Treasury"
  - Signers:  Add the 3 addresses from Step 1
  - Threshold: 2 of 3
  Click "Create"

  The Safe UI returns the new Safe address:
  x3-grant-safe.eth (or your network-specific): 0x....

STEP 3 — VERIFY the deployment:

  - Check the Safe appears in your Safe UI account list
  - Open it; verify owner addresses are the 3 you intended
  - Verify threshold = 2
  - Send a tiny test transaction (1 wei or equivalent)
  - Have 2 different hardware wallets sign it (different machines is fine)
  - Confirm execute + verify it lands

STEP 4 — For each chain you want the same multisig on:

  - Switch network in Safe UI → repeat "Create new Safe" on that chain
    (Safe does NOT auto-deploy cross-chain)
  - Networks of interest: Ethereum mainnet, Base, Optimism, Arbitrum, Polygon

STEP 5 — RECORD (in password manager, never in this repo):

  - Safe address(es) per chain
  - Safe UI URL
  - Owner addresses
  - Threshold

WHAT NOT TO DO:
  - Never paste hardware wallet seed phrases into ANY online machine
  - Never paste them into this repo
  - Never give more than 2 co-signers access to the same seed plate copy

EOF
    ;;

  solana)
    cat <<'EOF'

==========================================================================
SOLANA GRANT SQUADS (only if you ever apply to a Solana Foundation grant)
==========================================================================

Use Squads Protocol (squads.so) for multi-sig on Solana. Same idea as
Safe, but Solana-native.

STEP 1 — Generate 3 Solana keypairs on AIR-GAPPED machine:

  # Install solana CLI from https://docs.solana.com/cli/install-solana-cli-tools
  solana-keygen new --no-bip39-passphrase --silent --outfile signer1.json
  solana-keygen new --no-bip39-passphrase --silent --outfile signer2.json
  solana-keygen new --no-bip39-passphrase --silent --outfile signer3.json

  solana-keygen pubkey signer1.json    # founder
  solana-keygen pubkey signer2.json    # co-signer 1
  solana-keygen pubkey signer3.json    # co-signer 2

  Record 3 secret keys as 64-byte hex arrays (or base58 JSON arrays) on
  steel / encrypted backup. Three physical copies, two locations.

STEP 2 — On a TRUSTED machine (not this repo machine):

  Open https://squads.so  → "Create Multisig"
  - Wallet: connect Phantom / Backpack / Solflare
  - Members: paste the 3 pubkeys from Step 1
  - Threshold: 2
  - Name: "X3-Grant-Treasury"

  Record the multisig address (base58):
  x3-grant-sol: <base58 address>

  Fund it with a tiny amount of SOL (~0.01) to activate the account.

STEP 3 — Verify by sending a tiny test transfer + 2-of-3 signing.

EOF
    ;;

  *)
    usage
    exit 1
    ;;
esac

cat <<'BOTTOM'

==========================================================================
FINAL CHECKLIST (do BEFORE pasting any address into a grant application):
==========================================================================

  [ ]  Air-gapped machine used for key generation (not this one)
  [ ]  3 mnemonics each recorded on steel, 2 physical locations
  [ ]  Each signer loaded into a hardware wallet
  [ ]  Hardware-wallet-derived addresses match Step 3 outputs
  [ ]  Multisig setup UI shows correct owner set + correct threshold
  [ ]  Test transaction created, signed by 2-of-3, executed on-chain
  [ ]  Multisig address recorded in password manager
  [ ]  Multisig address NOT pasted into this repo, into a private git,
       into a Notion page, into a cloud doc, into a screenshot, or into
       any context that gets sent to a model API
  [ ]  Treasury policy doc reviewed for the network: TREASURY_POLICY.md

==========================================================================
BOTTOM
