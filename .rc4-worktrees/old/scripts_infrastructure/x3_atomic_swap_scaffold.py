#!/usr/bin/env python3
"""
Deterministic scaffold for X3 cross-chain atomic swaps (X3VM -> EVM/SVM/BTC).

This is a runnable, in-memory flow meant for developer understanding.
It does not connect to external chains.
"""
from __future__ import annotations

import hashlib
import hmac
import struct
import time
from dataclasses import dataclass


def sha256(data: bytes) -> bytes:
    return hashlib.sha256(data).digest()


def hex32(b: bytes) -> str:
    return b.hex()


@dataclass(frozen=True)
class Proof:
    trade_id: bytes
    mirror_chain_id: bytes
    action: str  # MINT | BURN | UNLOCK | REFUND
    amount: int
    recipient: bytes
    preimage_hash: bytes
    nonce: int
    signature: bytes


class ProofCodec:
    """Canonical proof encoder (deterministic)."""

    @staticmethod
    def encode_without_signature(
        trade_id: bytes,
        mirror_chain_id: bytes,
        action: str,
        amount: int,
        recipient: bytes,
        preimage_hash: bytes,
        nonce: int,
    ) -> bytes:
        # Fixed field order, fixed endianness, length‑prefixed recipient
        action_b = action.encode("ascii")
        if len(action_b) > 8:
            raise ValueError("action too long")
        action_b = action_b.ljust(8, b"\x00")
        return b"".join([
            trade_id,
            mirror_chain_id,
            action_b,
            struct.pack("<Q", amount),
            struct.pack("<H", len(recipient)),
            recipient,
            preimage_hash,
            struct.pack("<Q", nonce),
        ])

    @staticmethod
    def proof_hash(
        trade_id: bytes,
        mirror_chain_id: bytes,
        action: str,
        amount: int,
        recipient: bytes,
        preimage_hash: bytes,
        nonce: int,
    ) -> bytes:
        data = ProofCodec.encode_without_signature(
            trade_id,
            mirror_chain_id,
            action,
            amount,
            recipient,
            preimage_hash,
            nonce,
        )
        return sha256(data)


class ThresholdSigner:
    """Scaffold signer (HMAC), stands in for threshold aggregation."""

    def __init__(self, key: bytes) -> None:
        self.key = key

    def sign(self, proof_hash: bytes) -> bytes:
        return hmac.new(self.key, proof_hash, hashlib.sha256).digest()

    def verify(self, proof_hash: bytes, signature: bytes) -> bool:
        return hmac.compare_digest(self.sign(proof_hash), signature)


class ProofRegistry:
    def __init__(self) -> None:
        self.seen: set[bytes] = set()
        self.nonces: dict[bytes, int] = {}

    def next_nonce(self, chain_id: bytes) -> int:
        n = self.nonces.get(chain_id, 0) + 1
        self.nonces[chain_id] = n
        return n

    def check_replay(self, proof_hash: bytes) -> bool:
        if proof_hash in self.seen:
            return False
        self.seen.add(proof_hash)
        return True


@dataclass
class SwapLeg:
    chain_id: bytes
    recipient: bytes
    amount: int
    timeout_seconds: int


class CanonicalLedger:
    def __init__(self) -> None:
        self.balances: dict[bytes, int] = {}
        self.locked: dict[bytes, int] = {}

    def credit(self, account: bytes, amount: int) -> None:
        self.balances[account] = self.balances.get(account, 0) + amount

    def lock(self, account: bytes, amount: int) -> None:
        if self.balances.get(account, 0) < amount:
            raise ValueError("insufficient canonical balance")
        self.balances[account] -= amount
        self.locked[account] = self.locked.get(account, 0) + amount

    def unlock(self, account: bytes, amount: int) -> None:
        if self.locked.get(account, 0) < amount:
            raise ValueError("insufficient locked balance")
        self.locked[account] -= amount
        self.balances[account] = self.balances.get(account, 0) + amount


class MirrorChain:
    def __init__(self, chain_id: bytes, signer: ThresholdSigner, registry: ProofRegistry) -> None:
        self.chain_id = chain_id
        self.signer = signer
        self.registry = registry
        self.balances: dict[bytes, int] = {}
        self.escrow: dict[bytes, tuple[int, bytes, float, bytes]] = {}

    def _verify_proof(self, proof: Proof) -> None:
        proof_hash = ProofCodec.proof_hash(
            proof.trade_id,
            proof.mirror_chain_id,
            proof.action,
            proof.amount,
            proof.recipient,
            proof.preimage_hash,
            proof.nonce,
        )
        if not self.signer.verify(proof_hash, proof.signature):
            raise ValueError("invalid signature")
        if not self.registry.check_replay(proof_hash):
            raise ValueError("replay detected")

    def lock_with_proof(self, trade_id: bytes, proof: Proof, hashlock: bytes, timeout_seconds: int) -> None:
        self._verify_proof(proof)
        deadline = time.time() + timeout_seconds
        self.escrow[trade_id] = (proof.amount, hashlock, deadline, proof.recipient)

    def unlock(self, trade_id: bytes, preimage: bytes) -> None:
        if trade_id not in self.escrow:
            raise ValueError("no escrow")
        amount, hashlock, deadline, recipient = self.escrow[trade_id]
        if time.time() > deadline:
            raise ValueError("timeout exceeded")
        if sha256(preimage) != hashlock:
            raise ValueError("invalid preimage")
        self.balances[recipient] = self.balances.get(recipient, 0) + amount
        del self.escrow[trade_id]

    def refund(self, trade_id: bytes) -> None:
        if trade_id not in self.escrow:
            return
        _amount, _hashlock, deadline, _recipient = self.escrow[trade_id]
        if time.time() <= deadline:
            raise ValueError("not expired")
        del self.escrow[trade_id]
        # Refund emitted (no balance change in this scaffold)

    def burn(self, owner: bytes, amount: int) -> None:
        if self.balances.get(owner, 0) < amount:
            raise ValueError("insufficient mirror balance")
        self.balances[owner] -= amount


class SwapCoordinator:
    def __init__(self, ledger: CanonicalLedger, signer: ThresholdSigner, registry: ProofRegistry) -> None:
        self.ledger = ledger
        self.signer = signer
        self.registry = registry

    def mint_proof(self, trade_id: bytes, chain_id: bytes, recipient: bytes, amount: int, preimage_hash: bytes) -> Proof:
        nonce = self.registry.next_nonce(chain_id)
        proof_hash = ProofCodec.proof_hash(
            trade_id,
            chain_id,
            "MINT",
            amount,
            recipient,
            preimage_hash,
            nonce,
        )
        sig = self.signer.sign(proof_hash)
        return Proof(trade_id, chain_id, "MINT", amount, recipient, preimage_hash, nonce, sig)

    def burn_proof(self, trade_id: bytes, chain_id: bytes, recipient: bytes, amount: int, preimage_hash: bytes) -> Proof:
        nonce = self.registry.next_nonce(chain_id)
        proof_hash = ProofCodec.proof_hash(
            trade_id,
            chain_id,
            "BURN",
            amount,
            recipient,
            preimage_hash,
            nonce,
        )
        sig = self.signer.sign(proof_hash)
        return Proof(trade_id, chain_id, "BURN", amount, recipient, preimage_hash, nonce, sig)

    def create_swap(self, owner: bytes, trade_id: bytes, legs: list[SwapLeg], preimage: bytes) -> bytes:
        total = sum(leg.amount for leg in legs)
        self.ledger.lock(owner, total)
        hashlock = sha256(preimage)
        return hashlock


# -----------------------------------------------------------------------------
# Approved Parameters (Scaffold Only)
# -----------------------------------------------------------------------------

ASSET_ID = 1000
SYMBOL = "X3"
DECIMALS = 18
TOTAL_SUPPLY = 8_888_888_888

TARGET_TPS = 2_000_000_000
MAX_BATCH_PER_VALIDATOR = 50_000
SWAP_PARALLELISM = 100_000
PROOF_EMIT_INTERVAL_MS = 10
GPU_NODES_REQUIRED = 128


# -----------------------------------------------------------------------------
# X3-Lang-like Primitive Hooks (Scaffold)
# -----------------------------------------------------------------------------

def deposit_and_mint(
    coordinator: SwapCoordinator,
    chain: MirrorChain,
    owner: bytes,
    recipient: bytes,
    amount: int,
    trade_id: bytes,
    preimage: bytes,
    timeout_seconds: int,
) -> bytes:
    hashlock = coordinator.create_swap(
        owner,
        trade_id,
        [SwapLeg(chain.chain_id, recipient, amount, timeout_seconds)],
        preimage,
    )
    proof = coordinator.mint_proof(trade_id, chain.chain_id, recipient, amount, hashlock)
    chain.lock_with_proof(trade_id, proof, hashlock, timeout_seconds)
    return hashlock


def execute_atomic_swap(
    chains: list[MirrorChain],
    trade_id: bytes,
    preimage: bytes,
) -> None:
    for chain in chains:
        chain.unlock(trade_id, preimage)


def burn_and_exit(
    coordinator: SwapCoordinator,
    chain: MirrorChain,
    owner: bytes,
    recipient: bytes,
    amount: int,
    trade_id: bytes,
    hashlock: bytes,
) -> None:
    chain.burn(recipient, amount)
    _burn_proof = coordinator.burn_proof(trade_id, chain.chain_id, recipient, amount, hashlock)
    coordinator.ledger.unlock(owner, amount)


def batch_execute(actions: list[callable]) -> None:
    for action in actions:
        action()


def stress_test_loop(iterations: int, actions: list[callable]) -> None:
    for _ in range(iterations):
        batch_execute(actions)
        time.sleep(PROOF_EMIT_INTERVAL_MS / 1000.0)


def demo() -> None:
    signer = ThresholdSigner(b"x3-threshold-scaffold-key")
    registry = ProofRegistry()
    ledger = CanonicalLedger()
    coordinator = SwapCoordinator(ledger, signer, registry)

    alice = b"alice"
    ledger.credit(alice, 100)

    evm = MirrorChain(b"EVM", signer, registry)
    svm = MirrorChain(b"SVM", signer, registry)
    btc = MirrorChain(b"BTC", signer, registry)

    preimage = b"super-secret"
    trade_id = sha256(b"trade-1")

    # Deposit canonical X3 and mint mirrors
    hashlock = deposit_and_mint(coordinator, evm, alice, b"alice-evm", 50, trade_id, preimage, 12)
    deposit_and_mint(coordinator, svm, alice, b"alice-svm", 30, trade_id, preimage, 6)
    deposit_and_mint(coordinator, btc, alice, b"alice-btc", 20, trade_id, preimage, 48)

    # Execute atomic swap across chains
    execute_atomic_swap([evm, svm, btc], trade_id, preimage)

    # Burn mirror tokens and unlock canonical
    burn_and_exit(coordinator, evm, alice, b"alice-evm", 50, trade_id, hashlock)
    burn_and_exit(coordinator, svm, alice, b"alice-svm", 30, trade_id, hashlock)
    burn_and_exit(coordinator, btc, alice, b"alice-btc", 20, trade_id, hashlock)

    print("Canonical balance:", ledger.balances.get(alice, 0))
    print("Canonical locked:", ledger.locked.get(alice, 0))
    print("Stress target TPS:", TARGET_TPS)


if __name__ == "__main__":
    demo()
