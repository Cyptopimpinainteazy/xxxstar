# X3 Rust VM Bridge Proof Schema

This document describes the JSON proof packets accepted by the Rust `x3-lang-vm`
production bridge verifiers. These packets are carried in `Operation::Bridge`
payload fields:

- `source_finality_proof`
- `transfer_proof`

Both fields are UTF-8 JSON bytes when using the built-in light-client verifiers.

## Ethereum Header Proof

`EthereumLightClientVerifier::verify_evm_finality` accepts:

```json
{
  "proof_type": "ethereum-header-rlp-v1",
  "rlp_header": "0x...",
  "header_hash": "0x...",
  "receipts_root": "0x..."
}
```

Rules:

- `header_hash` must equal `keccak256(rlp_header)`.
- `header_hash` must equal the verifier's configured trusted finalized header hash.
- `receipts_root`, when supplied, must match the RLP header `receiptsRoot` field.
- The verifier extracts block number from the RLP header and enforces the optional minimum block number.

## Ethereum Receipt Trie Proof

`EthereumLightClientVerifier::verify_evm_transfer_proof` accepts:

```json
{
  "proof_type": "ethereum-receipt-trie-v1",
  "receipt_key": "0x...",
  "receipt_rlp": "0x...",
  "receipt_hash": "0x...",
  "receipts_root": "0x...",
  "trie_nodes": ["0x...", "0x..."],
  "log": {
    "address": "0x...",
    "topics": ["0xddf252ad...", "0x...", "0x..."],
    "data": "0x..."
  }
}
```

Rules:

- `receipt_hash`, when supplied, must equal `keccak256(receipt_rlp)`.
- `trie_nodes` are RLP-encoded Ethereum hexary Merkle Patricia Trie nodes ordered root to leaf.
- `receipt_key` is the trie key for the receipt, normally the RLP-encoded transaction index bytes.
- The verifier walks branch, extension, and leaf nodes, decodes hex-prefix paths, verifies node references by inline RLP or Keccak hash, and proves `receipt_rlp` against the finalized `receiptsRoot`.
- Legacy typed receipts are supported as `type_byte || rlp(receipt_payload)`.
- `log` must prove the exact ERC-20 transfer token address, receiver topic, and amount when `with_erc20_transfer_event` is configured.

## Solana Bank Proof

`SolanaLightClientVerifier::verify_svm_finality` accepts:

```json
{
  "proof_type": "solana-bank-hash-v1",
  "slot": 123,
  "bank_hash": "0x...",
  "parent_bank_hash": "0x...",
  "signatures": [
    {
      "public_key": "0x...",
      "signature": "0x..."
    }
  ]
}
```

Rules:

- `bank_hash` must equal the verifier's configured trusted finalized bank hash.
- Each signature is Ed25519 over:
  `solana-bank-hash-v1:<slot>:<bank_hash>:<parent_bank_hash>`.
- Without an `epoch_proof`, this is the legacy static-validator mode: if validator public keys are configured, signatures from other keys are ignored, and the configured minimum signature count must be met.
- With an `epoch_proof`, the bank signatures are verified against the active epoch validator set and must meet the configured stake threshold.

### Solana Epoch/Stake Proof

Stake-weighted Solana bank finality adds an `epoch_proof` object to the bank
proof:

```json
{
  "proof_type": "solana-bank-hash-v1",
  "slot": 123,
  "bank_hash": "0x...",
  "parent_bank_hash": "0x...",
  "signatures": [
    {
      "public_key": "0x...",
      "signature": "0x..."
    }
  ],
  "epoch_proof": {
    "proof_type": "solana-epoch-stake-v1",
    "epoch": 99,
    "parent_epoch_hash": "0x...",
    "epoch_hash": "0x...",
    "bank_accounts_root": "0x...",
    "stake_accounts": [
      {
        "proof_type": "solana-stake-account-v1",
        "account_pubkey": "0x...",
        "data_encoding": "solana-bincode-stake-state-v2",
        "data": "0x...",
        "data_hash": "0x...",
        "proof": {
          "account_pubkey": "0x...",
          "owner": "Stake11111111111111111111111111111111111111",
          "lamports": "700000000000",
          "data_hash": "0x...",
          "account_hash": "0x...",
          "bank_accounts_root": "0x...",
          "merkle_proof": [
            {
              "direction": "right",
              "sibling": "0x..."
            }
          ]
        }
      }
    ],
    "transition": {
      "proof_type": "solana-epoch-transition-v1",
      "parent_epoch_hash": "0x...",
      "validators": [
        {
          "public_key": "0x...",
          "stake": "900000000000",
          "active": true
        }
      ],
      "signatures": [
        {
          "public_key": "0x...",
          "signature": "0x..."
        }
      ]
    }
  }
}
```

Rules:

- `epoch_hash` must equal `sha256` over the canonical epoch, parent epoch hash, and sorted validator set.
- The verifier must be configured with `with_trusted_epoch_hash(epoch, epoch_hash)`; otherwise stake-weighted epoch proofs are rejected as unanchored.
- `stake_accounts` are the preferred source of validator stake. Each entry must provide hex-encoded account `data`, `data_encoding`, a matching `data_hash`, and a `proof` object whose account hash is recomputed from account pubkey, owner, lamports, and data hash.
- For `stake_accounts`, epoch-level `bank_accounts_root`, per-account `proof.bank_accounts_root`, and `proof.merkle_proof` are required. Each Merkle path must connect the recomputed account hash to the epoch bank/accounts root.
- `data_encoding: "solana-bincode-stake-state-v2"` decodes Solana stake-program `StakeStateV2::Stake` binary data and derives `voter_pubkey`, delegated stake, activation epoch, and deactivation epoch from the account bytes.
- `data_encoding: "x3-json-fixture-v1"` remains as an explicit legacy fixture fallback, but new fixtures should not use it.
- Active validator stakes are summed as `u128`, duplicate stake accounts and duplicate voter keys are rejected, and inactive validators do not count toward bank signature stake.
- A legacy `validators` array is still accepted for old fixtures that do not provide `stake_accounts`, but new stake-weighted proofs should use account-derived stake.
- Bank signatures are Ed25519 over `solana-bank-hash-v1:<slot>:<bank_hash>:<parent_bank_hash>` and must represent at least the configured stake threshold. The default threshold is 6,667 basis points.
- If `transition` is present, parent epoch validators sign `solana-epoch-transition-v1:<epoch>:<parent_epoch_hash>:<epoch_hash>`, and those signatures must also meet the configured stake threshold.

## Solana Transaction Proof

`SolanaLightClientVerifier::verify_svm_transfer_proof` accepts:

```json
{
  "proof_type": "solana-transaction-proof-v1",
  "slot": 123,
  "bank_hash": "0x...",
  "message": "0x...",
  "transaction_hash": "0x...",
  "signatures": [
    {
      "public_key": "0x...",
      "signature": "0x..."
    }
  ],
  "instructions": [
    {
      "programId": "Tokenkeg...",
      "parsed": {
        "type": "transferChecked",
        "info": {
          "mint": "Mint...",
          "destination": "Receiver...",
          "tokenAmount": {
            "amount": "42"
          }
        }
      }
    }
  ]
}
```

Rules:

- `slot` and `bank_hash` must match the verified bank proof receipt.
- `transaction_hash`, when supplied, must equal `sha256(message)`.
- Each transaction signature is Ed25519 over `message`.
- Parsed transfer instruction fields must match the bridge request asset, receiver, amount, and optional program id.

## Current Limits

- Ethereum proof validation covers receipt trie inclusion and typed receipt payloads, but fixture coverage is still generated in unit tests rather than imported from live chain proof archives.
- Solana proof validation now derives stake from verifiable stake-account fixture data and enforces optional parent-epoch transition signatures, but it still does not implement a full Solana consensus light client for Tower BFT vote lockouts, shred/bank fork choice, real AccountsDb append-vec proofs, or live stake-program account binary decoding.

## Ethereum Fixture Generator

Use `scripts/proof/generate_eth_bridge_fixture.py` to produce a fixture from
Ethereum RPC data plus an archived receipt trie proof:

```bash
python3 scripts/proof/generate_eth_bridge_fixture.py \
  --rpc-url "$ETH_RPC_URL" \
  --tx-hash 0x... \
  --receipt-proof-archive receipt-proof-archive.json \
  --token-address 0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48 \
  --receiver 0x1111111111111111111111111111111111111111 \
  --amount 42 \
  --output docs/x3-lang/fixtures/ethereum-receipt-trie-proof.fixture.json
```

The archive file must contain:

```json
{
  "rlp_header": "0x...",
  "receipt_key": "0x...",
  "receipt_rlp": "0x...",
  "receipt_hash": "0x...",
  "trie_nodes": ["0x..."]
}
```

To build an archive from a block's full receipt set, use the checked-in Rust
builder:

```bash
cargo run --manifest-path x3-lang/Cargo.toml \
  -p x3-tools --bin build_eth_receipt_archive -- \
  --rpc-url "$ETH_RPC_URL" \
  --block 17000000 \
  --tx-hash 0x0c0093106cb919958037a68aa3ce785a4c9a8ff430f4e49fd0c32a6348ee1c46 \
  > docs/x3-lang/fixtures/ethereum-mainnet-17000000-usdc-receipt-proof.archive.json
```

For offline reproduction, save a JSON object with `block` and `receipts` fields
and pass `--input-json <path>` instead of `--rpc-url/--block`.

The repository includes a pinned full-block receipt input for CI regeneration:

```bash
cargo run --manifest-path x3-lang/Cargo.toml \
  -p x3-tools --bin build_eth_receipt_archive -- \
  --input-json docs/x3-lang/fixtures/ethereum-full-block-receipts/mainnet-17000000-receipts.json \
  --tx-hash 0x3b72731ebb4192b8307641d480078ffaaa362402f11ee491022ee5fb0672db02
```

The `x3 bridge fixture regeneration` workflow rebuilds the bundled block
17000000 USDC, failed-receipt, and multi-log archives from that pinned input,
then runs them through the fixture generator and VM verifier tests.

The repository also includes
`docs/x3-lang/fixtures/solana-epoch-stake-account-proof.fixture.json`, a small
Solana epoch fixture whose active stake is derived from verified stake-account
data instead of a caller-supplied validator list. The VM test
`svm_epoch_stake_account_fixture_decodes_to_active_stake` loads this fixture and
checks the derived active stake.

The tool fetches `eth_getTransactionReceipt` and `eth_getBlockByHash`, extracts
the matching ERC-20 transfer log, imports the archived receipt trie nodes, and
emits `source_finality_proof` plus `transfer_proof` JSON.

Validate an existing fixture without RPC:

```bash
python3 scripts/proof/generate_eth_bridge_fixture.py \
  --validate-only docs/x3-lang/fixtures/ethereum-receipt-trie-proof.fixture.json
```

Emit a fixture from a bundled archive without RPC:

```bash
python3 scripts/proof/generate_eth_bridge_fixture.py \
  --from-archive-only docs/x3-lang/fixtures/ethereum-mainnet-46147-receipt-proof.archive.json
```

Important: stock Ethereum JSON-RPC does not expose receipt trie proof nodes.
`eth_getProof` proves account/storage state, not transaction receipts. The
`trie_nodes` archive must come from an archive node plugin, proof service, or
offline receipt-trie builder.

The repository includes captured archive fixtures:

- `docs/x3-lang/fixtures/ethereum-mainnet-46147-receipt-proof.archive.json`
  captures Ethereum mainnet block `46147`, transaction
  `0x5c504ed432cb51138bcf09aa5e8a410dd4a1e204ef84bfed1be16dfba1b22060`,
  its pre-Byzantium receipt RLP, receipt key, and receipt trie node. The VM test
  `ethereum_mainnet_archive_fixture_runs_through_generator_and_vm_verifier`
  runs this archive through the generator and verifier.
- `docs/x3-lang/fixtures/ethereum-mainnet-17000000-usdc-receipt-proof.archive.json`
  captures Ethereum mainnet block `17000000`, transaction
  `0x0c0093106cb919958037a68aa3ce785a4c9a8ff430f4e49fd0c32a6348ee1c46`,
  a type `0x02` USDC ERC-20 transfer receipt, receipt key `0x23`, and a
  three-node receipt trie proof reconstructed from the full block receipt set.
  The VM test `ethereum_modern_usdc_archive_verifies_erc20_transfer_event`
  runs this archive through the generator and verifies it with
  `with_erc20_transfer_event`.
- `docs/x3-lang/fixtures/ethereum-mainnet-17000000-failed-receipt-proof.archive.json`
  captures failed type `0x02` transaction
  `0x3b72731ebb4192b8307641d480078ffaaa362402f11ee491022ee5fb0672db02`
  from the same block. The VM test proves trie inclusion, then verifies that
  failed receipt status is rejected.
- `docs/x3-lang/fixtures/ethereum-mainnet-17000000-multilog-receipt-proof.archive.json`
  captures multi-log type `0x02` transaction
  `0xa278205118a242c87943b9ed83aacafe9906002627612ac3672d8ea224e38181`
  from the same block. The VM test runs it through the generator and verifies
  the first ERC-20 transfer log with `with_erc20_transfer_event`.
