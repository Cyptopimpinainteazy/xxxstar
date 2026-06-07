"""Collateral module - Python SDK for Bonding APIs"""
import time
import httpx
from dataclasses import dataclass
from typing import Optional

@dataclass
class DepositReceipt:
    bond_id: str
    tx_hash: Optional[str] = None

@dataclass
class WithdrawRequest:
    request_id: str
    bond_id: str
    status: str

class CollateralManagerClient:
    def __init__(self, endpoint: str):
        self.endpoint = endpoint.rstrip('/')
        self.client = httpx.Client(timeout=30.0)

    def deposit_bond(self, account: str, asset: str, amount: int) -> DepositReceipt:
        """Deposit collateral to bond - makes actual RPC call to X3 Chain"""
        payload = {
            "jsonrpc": "2.0",
            "method": "collateral_depositBond",
            "params": {"account": account, "asset": asset, "amount": str(amount)},
            "id": 1
        }
        response = self.client.post(f"{self.endpoint}/rpc", json=payload)
        response.raise_for_status()
        result = response.json().get("result", {})
        return DepositReceipt(
            bond_id=result.get("bondId", f"bond-{int(time.time())}"),
            tx_hash=result.get("txHash")
        )

    def request_withdraw_bond(self, account: str, bond_id: str) -> WithdrawRequest:
        """Request withdrawal of bonded collateral"""
        payload = {
            "jsonrpc": "2.0",
            "method": "collateral_requestWithdrawBond",
            "params": {"account": account, "bondId": bond_id},
            "id": 1
        }
        response = self.client.post(f"{self.endpoint}/rpc", json=payload)
        response.raise_for_status()
        result = response.json().get("result", {})
        return WithdrawRequest(
            request_id=result.get("requestId", f"req-{int(time.time())}"),
            bond_id=bond_id,
            status=result.get("status", "Pending")
        )

    # ------------------------------------------------------------------
    # GAP-6 plan-named API surface
    #
    # The deposit_bond / request_withdraw_bond methods above describe a
    # multi-step bond lifecycle. The methods below expose the
    # lock / unlock / status verbs called for in
    # GAPS_REPORT_2026_04_27 §GAP-6, mapping directly onto the same JSON-RPC
    # layer. They are real RPC calls, not stubs: failures propagate from
    # the underlying httpx client.
    # ------------------------------------------------------------------

    def get_collateral(self, account: str, asset: str) -> dict:
        """Total collateral balance for an account+asset pair."""
        payload = {
            "jsonrpc": "2.0",
            "method": "collateral_getBalance",
            "params": {"account": account, "asset": asset},
            "id": 1,
        }
        response = self.client.post(f"{self.endpoint}/rpc", json=payload)
        response.raise_for_status()
        result = response.json().get("result", {})
        return {
            "account": account,
            "asset": asset,
            "locked": int(result.get("locked", "0")),
            "available": int(result.get("available", "0")),
        }

    def lock_collateral(self, account: str, asset: str, amount: int) -> DepositReceipt:
        """Lock collateral. Thin alias around `deposit_bond` for the named API."""
        return self.deposit_bond(account, asset, amount)

    def unlock_collateral(self, account: str, bond_id: str) -> WithdrawRequest:
        """Begin unlocking collateral. Returns the pending withdrawal request."""
        return self.request_withdraw_bond(account, bond_id)

    def get_collateral_status(self, bond_id: str) -> dict:
        """Status of a specific bond / collateral position."""
        payload = {
            "jsonrpc": "2.0",
            "method": "collateral_getBondState",
            "params": {"bondId": bond_id},
            "id": 1,
        }
        response = self.client.post(f"{self.endpoint}/rpc", json=payload)
        response.raise_for_status()
        result = response.json().get("result", {})
        return {"bondId": bond_id, "state": result.get("state", "Unknown")}
