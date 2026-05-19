"""
Jury Blockchain Anchorer

Anchors jury decisions to the blockchain via the x3-jury-anchor pallet.
"""

import asyncio
import hashlib
import json
import logging
import os
from typing import Any, Dict, Optional

import aiohttp
from pydantic import BaseModel

logger = logging.getLogger(__name__)


class JuryDecisionRecord(BaseModel):
    """On-chain jury decision record."""
    session_id: str
    decision_hash: str
    block_number: int
    timestamp: int
    jury_authority: str
    metadata: Dict[str, Any]


class AnchorResult(BaseModel):
    """Result of anchoring operation."""
    session_id: str
    decision_hash: str
    tx_hash: str
    block_number: Optional[int] = None
    status: str  # "pending", "anchored", "failed"


class JuryAnchorer:
    """Anchors jury decisions to blockchain."""
    
    def __init__(
        self,
        rpc_url: str,
        jury_manager_account: str,
        jury_authority_private_key: str,
        timeout: int = 30,
    ):
        """
        Initialize anchorer.
        
        Args:
            rpc_url: RPC endpoint URL
            jury_manager_account: Account ID for jury manager
            jury_authority_private_key: Private key for signing (hex)
            timeout: RPC request timeout in seconds
        """
        self.rpc_url = rpc_url
        self.account = jury_manager_account
        self.private_key = jury_authority_private_key
        self.timeout = timeout
        self.session: Optional[aiohttp.ClientSession] = None
    
    async def __aenter__(self):
        """Async context manager entry."""
        self.session = aiohttp.ClientSession()
        return self
    
    async def __aexit__(self, exc_type, exc_val, exc_tb):
        """Async context manager exit."""
        if self.session:
            await self.session.close()
    
    async def _rpc_call(self, method: str, params: list) -> Dict[str, Any]:
        """Make RPC call."""
        if not self.session:
            raise RuntimeError("Anchorer not in async context")
        
        payload = {
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": 1,
        }
        
        try:
            async with self.session.post(
                self.rpc_url,
                json=payload,
                timeout=aiohttp.ClientTimeout(total=self.timeout),
            ) as resp:
                data = await resp.json()
                
                if "error" in data:
                    raise RuntimeError(f"RPC error: {data['error']}")
                
                return data.get("result", {})
        except asyncio.TimeoutError:
            logger.error(f"RPC timeout calling {method}")
            raise
        except Exception as e:
            logger.error(f"RPC call failed: {e}")
            raise
    
    async def anchor_decision(
        self,
        session_id: str,
        decision_hash: str,
    ) -> AnchorResult:
        """
        Anchor jury decision to blockchain.
        
        Args:
            session_id: Unique session identifier
            decision_hash: SHA256 hash of decision (hex)
        
        Returns:
            AnchorResult with status and block number
        """
        logger.info(f"Anchoring decision {session_id} with hash {decision_hash}")
        
        # Build extrinsic call
        # Format: x3_jury_anchor.anchor_decision(session_id, decision_hash)
        
        extrinsic = {
            "pallet": "atlasJuryAnchor",
            "method": "anchorDecision",
            "args": {
                "session_id": session_id.encode().hex(),
                "decision_hash": decision_hash.lstrip("0x"),
            },
        }
        
        try:
            # Call submit_extrinsic RPC
            result = await self._rpc_call(
                "author_submitExtrinsic",
                [json.dumps(extrinsic)],
            )
            
            tx_hash = result.get("tx_hash")
            
            if not tx_hash:
                return AnchorResult(
                    session_id=session_id,
                    decision_hash=decision_hash,
                    tx_hash="",
                    status="failed",
                )
            
            # Poll for confirmation
            block_number = await self._wait_for_finalization(tx_hash)
            
            return AnchorResult(
                session_id=session_id,
                decision_hash=decision_hash,
                tx_hash=tx_hash,
                block_number=block_number,
                status="anchored" if block_number else "pending",
            )
        except Exception as e:
            logger.error(f"Failed to anchor decision: {e}")
            return AnchorResult(
                session_id=session_id,
                decision_hash=decision_hash,
                tx_hash="",
                status="failed",
            )
    
    async def _wait_for_finalization(
        self,
        tx_hash: str,
        max_retries: int = 30,
    ) -> Optional[int]:
        """Wait for transaction finalization."""
        for attempt in range(max_retries):
            try:
                result = await self._rpc_call(
                    "chain_getTransactionStatus",
                    [tx_hash],
                )
                
                if result.get("finalized"):
                    return result.get("block_number")
                
                await asyncio.sleep(1)
            except Exception as e:
                logger.debug(f"Status check failed (attempt {attempt}): {e}")
                await asyncio.sleep(1)
        
        logger.warning(f"Transaction {tx_hash} not finalized after {max_retries} attempts")
        return None
    
    async def verify_decision(
        self,
        session_id: str,
        expected_hash: str,
    ) -> bool:
        """
        Verify decision matches blockchain record.
        
        Args:
            session_id: Session identifier
            expected_hash: Expected decision hash
        
        Returns:
            True if blockchain hash matches expected
        """
        try:
            result = await self._rpc_call(
                "jury_decisionStatus",
                [session_id],
            )
            
            if result.get("status") != "anchored":
                logger.warning(f"Decision {session_id} not anchored")
                return False
            
            on_chain_hash = result.get("on_chain", {}).get("decision_hash")
            
            if not on_chain_hash:
                logger.error(f"No hash in on-chain record for {session_id}")
                return False
            
            matches = on_chain_hash.lstrip("0x") == expected_hash.lstrip("0x")
            
            if matches:
                logger.info(f"Decision {session_id} verified on blockchain")
            else:
                logger.error(
                    f"Hash mismatch for {session_id}: "
                    f"on-chain={on_chain_hash}, expected={expected_hash}"
                )
            
            return matches
        except Exception as e:
            logger.error(f"Verification failed: {e}")
            return False
    
    async def get_decision_status(
        self,
        session_id: str,
    ) -> Optional[JuryDecisionRecord]:
        """
        Get on-chain decision record.
        
        Args:
            session_id: Session identifier
        
        Returns:
            JuryDecisionRecord or None if not found
        """
        try:
            result = await self._rpc_call(
                "jury_decisionStatus",
                [session_id],
            )
            
            on_chain = result.get("on_chain")
            if not on_chain:
                return None
            
            return JuryDecisionRecord(
                session_id=session_id,
                decision_hash=on_chain["decision_hash"],
                block_number=on_chain["block_number"],
                timestamp=on_chain["timestamp"],
                jury_authority=on_chain.get("jury_authority", ""),
                metadata=on_chain.get("metadata", {}),
            )
        except Exception as e:
            logger.error(f"Failed to get decision status: {e}")
            return None


class JuryAnchoringService:
    """Service for jury anchoring with audit logging."""
    
    def __init__(
        self,
        anchorer: JuryAnchorer,
        audit_logger,  # AuditLogger instance
    ):
        """
        Initialize anchoring service.
        
        Args:
            anchorer: JuryAnchorer instance
            audit_logger: AuditLogger for logging events
        """
        self.anchorer = anchorer
        self.audit_logger = audit_logger
    
    async def finalize_and_anchor(
        self,
        session_id: str,
        votes: Dict[str, bool],
        result: bool,
    ) -> bool:
        """
        Finalize jury session and anchor decision.
        
        Args:
            session_id: Jury session ID
            votes: {member_id: vote_bool} dictionary
            result: Jury decision result (True = PASS, False = FAIL)
        
        Returns:
            True if successfully anchored
        """
        # Compute decision hash
        decision_data = {
            "session_id": session_id,
            "votes": votes,
            "result": result,
            "timestamp": asyncio.get_event_loop().time(),
        }
        
        decision_hash = hashlib.sha256(
            json.dumps(decision_data, sort_keys=True).encode()
        ).hexdigest()
        
        logger.info(f"Decision hash for {session_id}: {decision_hash}")
        
        # Try to anchor
        anchor_result = await self.anchorer.anchor_decision(
            session_id,
            decision_hash,
        )
        
        # Log to audit trail
        self.audit_logger.log_event(
            session_id=session_id,
            event_type="decision_finalized",
            event_data={
                "decision_hash": decision_hash,
                "result": "PASS" if result else "FAIL",
                "votes": votes,
                "on_chain": anchor_result.status == "anchored",
                "tx_hash": anchor_result.tx_hash,
                "block_number": anchor_result.block_number,
            },
        )
        
        if anchor_result.status == "anchored":
            # Verify
            verified = await self.anchorer.verify_decision(
                session_id,
                decision_hash,
            )
            
            if verified:
                self.audit_logger.log_event(
                    session_id=session_id,
                    event_type="decision_verified",
                    event_data={
                        "decision_hash": decision_hash,
                        "block_number": anchor_result.block_number,
                    },
                )
                logger.info(f"Decision {session_id} anchored and verified")
                return True
            else:
                logger.error(f"Verification failed for {session_id}")
                return False
        else:
            logger.warning(f"Failed to anchor decision {session_id}")
            return False


# Example usage
async def main():
    """Example: Anchor a jury decision."""
    anchorer = JuryAnchorer(
        rpc_url=os.getenv("ONCHAIN_RPC_URL", "http://localhost:9944"),
        jury_manager_account=os.getenv("JURY_MANAGER_ACCOUNT", "5GrwvaEF"),
        jury_authority_private_key=os.getenv("JURY_AUTHORITY_PRIVATE_KEY", ""),
    )
    
    session_id = "sess-example-001"
    decision_hash = hashlib.sha256(b"decision_data").hexdigest()
    
    async with anchorer:
        result = await anchorer.anchor_decision(session_id, decision_hash)
        print(f"Anchor result: {result}")
        
        verified = await anchorer.verify_decision(session_id, decision_hash)
        print(f"Verified: {verified}")


if __name__ == "__main__":
    logging.basicConfig(level=logging.INFO)
    asyncio.run(main())
