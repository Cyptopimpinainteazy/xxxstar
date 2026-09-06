"""
Integration tests for jury blockchain anchoring.
"""

import hashlib
import json
from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from swarm.jury.anchorer import (
    AnchorResult,
    JuryAnchorer,
    JuryAnchoringService,
)


class TestJuryAnchorer:
    """Test JuryAnchorer class."""

    @pytest.fixture
    async def anchorer(self):
        """Create anchorer instance."""
        anchorer = JuryAnchorer(
            rpc_url="http://localhost:9944",
            jury_manager_account="5GrwvaEF5zXb26Fz9rcQkQTQq5LaWNe5ia5gihQTJ4vj",
            jury_authority_private_key="0x" + "a" * 64,
        )
        yield anchorer

    @pytest.mark.asyncio
    async def test_anchor_decision_success(self, anchorer):
        """Test successful decision anchoring."""
        with patch.object(anchorer, '_rpc_call') as mock_rpc:
            mock_rpc.return_value = {
                "tx_hash": "0xabcd1234",
            }

            with patch.object(anchorer, '_wait_for_finalization') as mock_wait:
                mock_wait.return_value = 12345

                result = await anchorer.anchor_decision(
                    "sess-001",
                    "0x" + hashlib.sha256(b"test").hexdigest(),
                )

                assert result.status == "anchored"
                assert result.session_id == "sess-001"
                assert result.block_number == 12345
                mock_rpc.assert_called()
                mock_wait.assert_called()

    @pytest.mark.asyncio
    async def test_anchor_decision_rpc_failure(self, anchorer):
        """Test anchorer handles RPC failures."""
        with patch.object(anchorer, '_rpc_call') as mock_rpc:
            mock_rpc.side_effect = RuntimeError("RPC failed")

            result = await anchorer.anchor_decision(
                "sess-002",
                "0x" + hashlib.sha256(b"test2").hexdigest(),
            )

            assert result.status == "failed"

    @pytest.mark.asyncio
    async def test_verify_decision_match(self, anchorer):
        """Test verification succeeds for matching hash."""
        expected_hash = "0x" + hashlib.sha256(b"decision").hexdigest()

        with patch.object(anchorer, '_rpc_call') as mock_rpc:
            mock_rpc.return_value = {
                "status": "anchored",
                "on_chain": {
                    "decision_hash": expected_hash,
                    "block_number": 100,
                },
            }

            result = await anchorer.verify_decision("sess-003", expected_hash)

            assert result is True

    @pytest.mark.asyncio
    async def test_verify_decision_mismatch(self, anchorer):
        """Test verification fails for non-matching hash."""
        with patch.object(anchorer, '_rpc_call') as mock_rpc:
            mock_rpc.return_value = {
                "status": "anchored",
                "on_chain": {
                    "decision_hash": "0x" + "a" * 64,
                    "block_number": 100,
                },
            }

            result = await anchorer.verify_decision(
                "sess-004",
                "0x" + "b" * 64,
            )

            assert result is False

    @pytest.mark.asyncio
    async def test_get_decision_status_found(self, anchorer):
        """Test retrieving existing decision."""
        with patch.object(anchorer, '_rpc_call') as mock_rpc:
            mock_rpc.return_value = {
                "status": "anchored",
                "on_chain": {
                    "decision_hash": "0x" + "c" * 64,
                    "block_number": 200,
                    "timestamp": 1234567890,
                    "jury_authority": "5GrwvaEF",
                    "metadata": {
                        "member_count": 5,
                        "quorum_threshold": 66,
                        "result": True,
                        "session_duration_secs": 900,
                    },
                },
            }

            record = await anchorer.get_decision_status("sess-005")

            assert record is not None
            assert record.session_id == "sess-005"
            assert record.block_number == 200

    @pytest.mark.asyncio
    async def test_get_decision_status_not_found(self, anchorer):
        """Test retrieving non-existent decision."""
        with patch.object(anchorer, '_rpc_call') as mock_rpc:
            mock_rpc.return_value = {"status": "not_found"}

            record = await anchorer.get_decision_status("sess-nonexistent")

            assert record is None

    @pytest.mark.asyncio
    async def test_wait_for_finalization_success(self, anchorer):
        """Test waiting for transaction finalization."""
        with patch.object(anchorer, '_rpc_call') as mock_rpc:
            # First two calls return not finalized, third returns finalized
            mock_rpc.side_effect = [
                {"finalized": False},
                {"finalized": False},
                {"finalized": True, "block_number": 333},
            ]

            block = await anchorer._wait_for_finalization("0xtx123")

            assert block == 333

    @pytest.mark.asyncio
    async def test_wait_for_finalization_timeout(self, anchorer):
        """Test timeout waiting for finalization."""
        with patch.object(anchorer, '_rpc_call') as mock_rpc:
            mock_rpc.return_value = {"finalized": False}

            block = await anchorer._wait_for_finalization(
                "0xtx456",
                max_retries=3,
            )

            assert block is None


class TestJuryAnchoringService:
    """Test JuryAnchoringService class."""

    @pytest.fixture
    def service(self):
        """Create service instance."""
        anchorer = AsyncMock()
        audit_logger = MagicMock()
        service = JuryAnchoringService(anchorer, audit_logger)
        return service

    @pytest.mark.asyncio
    async def test_finalize_and_anchor_success(self, service):
        """Test successful finalization and anchoring."""
        service.anchorer.anchor_decision = AsyncMock(
            return_value=AnchorResult(
                session_id="sess-006",
                decision_hash="0x" + "d" * 64,
                tx_hash="0xtx789",
                block_number=444,
                status="anchored",
            )
        )

        service.anchorer.verify_decision = AsyncMock(return_value=True)

        votes = {"INF-1": True, "OPS-1": True, "SEC-1": False}
        result = await service.finalize_and_anchor(
            "sess-006",
            votes,
            True,  # PASS
        )

        assert result is True
        service.anchorer.anchor_decision.assert_called_once()
        service.anchorer.verify_decision.assert_called_once()
        service.audit_logger.log_event.assert_called()

    @pytest.mark.asyncio
    async def test_finalize_and_anchor_verification_failed(self, service):
        """Test fails if verification fails."""
        service.anchorer.anchor_decision = AsyncMock(
            return_value=AnchorResult(
                session_id="sess-007",
                decision_hash="0x" + "e" * 64,
                tx_hash="0xtx999",
                block_number=555,
                status="anchored",
            )
        )

        service.anchorer.verify_decision = AsyncMock(return_value=False)

        votes = {"INF-1": True, "OPS-1": False}
        result = await service.finalize_and_anchor(
            "sess-007",
            votes,
            False,  # FAIL
        )

        assert result is False

    @pytest.mark.asyncio
    async def test_finalize_and_anchor_pending(self, service):
        """Test handles pending anchor."""
        service.anchorer.anchor_decision = AsyncMock(
            return_value=AnchorResult(
                session_id="sess-008",
                decision_hash="0x" + "f" * 64,
                tx_hash="0xtx111",
                status="pending",
            )
        )

        votes = {"member-1": True}
        result = await service.finalize_and_anchor(
            "sess-008",
            votes,
            True,
        )

        assert result is False


class TestJuryAnchoringEndToEnd:
    """End-to-end integration tests."""

    @pytest.mark.asyncio
    async def test_complete_jury_flow(self):
        """Test complete jury → anchor → verify flow."""
        # Simulate jury votes
        votes = {
            "INF-1": True,
            "INF-2": True,
            "OPS-1": True,
            "OPS-2": True,
            "SEC-1": False,
        }

        # Compute decision
        yes_votes = sum(1 for v in votes.values() if v)
        total = len(votes)
        threshold = 0.66
        decision = yes_votes / total > threshold  # True (4/5 = 80%)

        # Compute hash
        decision_data = {
            "votes": votes,
            "result": decision,
        }
        decision_hash = hashlib.sha256(
            json.dumps(decision_data, sort_keys=True).encode()
        ).hexdigest()

        # Mock anchorer
        anchorer = AsyncMock()
        anchorer.anchor_decision = AsyncMock(
            return_value=AnchorResult(
                session_id="sess-e2e",
                decision_hash="0x" + decision_hash,
                tx_hash="0xtxe2e",
                block_number=6666,
                status="anchored",
            )
        )
        anchorer.verify_decision = AsyncMock(return_value=True)

        # Run flow
        result = await anchorer.anchor_decision("sess-e2e", "0x" + decision_hash)

        assert result.status == "anchored"
        assert result.block_number == 6666

        verified = await anchorer.verify_decision("sess-e2e", "0x" + decision_hash)
        assert verified is True


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
