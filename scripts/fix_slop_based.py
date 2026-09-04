#!/usr/bin/env python3
"""Replace '-based' compounds and 'based on' phrases flagged by slop detector."""
import os

# Match current state after em-dash replacement
BASED_FIXES = [
    ("crates/x3-atomic-swap/src/x3vm_htlc.rs", [
        (3, 'X3VM-based chains', 'X3VM chains'),
    ]),
    ("crates/x3-atomic-swap/src/ton_htlc.rs", [
        (3, 'TVM-based', 'TVM'),
    ]),
    ("crates/x3-atomic-swap/src/substrate_htlc.rs", [
        (3, 'Substrate-based chains', 'Substrate chains'),
        (25, 'Substrate-based chains', 'Substrate chains'),
    ]),
    ("crates/x3-atomic-swap/src/polkadot_ink_htlc.rs", [
        (314, 'Substrate-based chains', 'Substrate chains'),
    ]),
    ("crates/x3-atomic-swap/src/near_htlc.rs", [
        (3, 'WASM-based', 'WASM'),
    ]),
    ("crates/x3-atomic-swap/src/move_vm_htlc.rs", [
        (3, 'MoveVM-based chains', 'MoveVM chains'),
        (8, 'checkpoint-based models', 'checkpoint models'),
        (98, 'MoveVM-based chains', 'MoveVM chains'),
    ]),
    ("crates/x3-atomic-swap/src/cosmwasm_htlc.rs", [
        (3, 'CosmWasm-based chains', 'CosmWasm chains'),
        (100, 'CosmWasm-based chains', 'CosmWasm chains'),
    ]),
    ("crates/x3-atomic-swap/src/cairo_vm_htlc.rs", [
        (3, 'CairoVM-based chains', 'CairoVM chains'),
        (52, 'CairoVM-based chains', 'CairoVM chains'),
    ]),
    ("crates/x3-atomic-swap/src/bitcoin_htlc.rs", [
        (3, 'Bitcoin-based chains', 'Bitcoin chains'),
        (444, 'based on intent data', 'from intent data'),
    ]),
    ("crates/x3-atomic-swap/src/adapter_ledger.rs", [
        (236, 'based on', 'depending on'),
        (245, 'based on the kind', 'by the kind'),
        (313, 'based on what has been', 'from what has been'),
    ]),
    ("crates/x3-atomic-swap/src/fuel_htlc.rs", [
        (3, 'FuelVM-based chains', 'FuelVM chains'),
    ]),
    ("crates/x3-atomic-swap/src/plutus_htlc.rs", [
        (3, 'Plutus/eUTXO-based chains', 'Plutus/eUTXO chains'),
    ]),
    ("crates/x3-atomic-swap/src/soroban_htlc.rs", [
        (87, 'Soroban WASM-based chains', 'Soroban WASM chains'),
    ]),
]

for fpath, replacements in BASED_FIXES:
    if not os.path.exists(fpath):
        print(f'MISSING: {fpath}')
        continue
    with open(fpath, 'r') as fh:
        lines = fh.readlines()
    changed = False
    for ln, old, new in replacements:
        idx = ln - 1
        if old in lines[idx]:
            lines[idx] = lines[idx].replace(old, new, 1)
            changed = True
            print(f'{fpath}:{ln}: FIXED')
        else:
            # Show actual line for debug
            print(f'{fpath}:{ln}: NOT FOUND expected "{old}"')
            print(f'  actual: {lines[idx].rstrip()[:120]}')
    if changed:
        with open(fpath, 'w') as fh:
            fh.writelines(lines)

print("\nDone.")