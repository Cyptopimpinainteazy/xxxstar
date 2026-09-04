#!/env/bin/python3
"""
Pilot Test Executor for X3 Chain Jury Service

This script orchestrates the pilot test scenarios defined in PILOT_PLAN.md:
- Session 1: Infrastructure Upgrade (5-member PASS scenario)
- Session 2: Security Policy (3-member FAIL scenario)

Usage:
    python pilot_executor.py --api-url http://localhost:8000 --scenario 1
    python pilot_executor.py --api-url http://localhost:8000 --scenario 2
    python pilot_executor.py --api-url http://localhost:8000 --scenario all
"""

import asyncio
import hashlib
import json
import time
import uuid
import logging
import sys
from datetime import datetime
from dataclasses import dataclass, asdict
from typing import List, Dict, Any, Optional
import argparse

try:
    import aiohttp
except ImportError:
    print("ERROR: aiohttp not installed. Run: pip install aiohttp")
    sys.exit(1)

# Logging setup
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)

# ============================================================
# Data Classes for Pilot Scenarios
# ============================================================

@dataclass
class JuryMember:
    """Jury member definition"""
    agent_id: str
    section: str
    is_on_chain: bool = False
    
    def to_dict(self):
        return asdict(self)

@dataclass
class PilotScenario:
    """Complete pilot test scenario"""
    name: str
    task_ids: List[str]
    members: List[JuryMember]
    votes: Dict[str, bool]  # agent_id -> vote (True/False)
    expected_result: bool
    commit_deadline_seconds: int = 300
    reveal_deadline_seconds: int = 600
    
    def get_expected_percentage(self) -> float:
        """Calculate expected yes percentage"""
        yes_count = sum(1 for vote in self.votes.values() if vote)
        total = len(self.votes)
        return yes_count / total if total > 0 else 0.0

# ============================================================
# Pilot Test Scenarios
# ============================================================

SCENARIO_1_INFRASTRUCTURE_UPGRADE = PilotScenario(
    name="Infrastructure Upgrade (PASS)",
    task_ids=["INFRA-UPGRADE-001"],
    members=[
        JuryMember("INF-001", "infrastructure"),
        JuryMember("INF-002", "infrastructure"),
        JuryMember("OPS-001", "operations"),
        JuryMember("OPS-002", "operations"),
        JuryMember("SEC-001", "security"),
    ],
    votes={
        "INF-001": True,
        "INF-002": True,
        "OPS-001": True,
        "OPS-002": True,
        "SEC-001": False,
    },
    expected_result=True,  # 4/5 = 80% > 66%
)

SCENARIO_2_SECURITY_POLICY = PilotScenario(
    name="Security Policy (FAIL)",
    task_ids=["SEC-POLICY-001"],
    members=[
        JuryMember("SEC-002", "security"),
        JuryMember("SEC-003", "security"),
        JuryMember("OPS-003", "operations"),
    ],
    votes={
        "SEC-002": False,
        "SEC-003": False,
        "OPS-003": True,
    },
    expected_result=False,  # 1/3 = 33% < 66%
)

SCENARIOS = {
    1: SCENARIO_1_INFRASTRUCTURE_UPGRADE,
    2: SCENARIO_2_SECURITY_POLICY,
}

# ============================================================
# Pilot Test Executor
# ============================================================

class PilotExecutor:
    def __init__(self, api_url: str):
        self.api_url = api_url.rstrip('/')
        self.session: Optional[aiohttp.ClientSession] = None
        self.results: List[Dict[str, Any]] = []
    
    async def __aenter__(self):
        self.session = aiohttp.ClientSession()
        return self
    
    async def __aexit__(self, exc_type, exc_val, exc_tb):
        if self.session:
            await self.session.close()
    
    async def check_health(self) -> bool:
        """Verify API is healthy"""
        try:
            async with self.session.get(f"{self.api_url}/health", timeout=5) as resp:
                if resp.status == 200:
                    logger.info("✅ API health check passed")
                    return True
                else:
                    logger.error(f"❌ API health check failed: {resp.status}")
                    return False
        except Exception as e:
            logger.error(f"❌ API health check error: {e}")
            return False
    
    async def create_session(self, scenario: PilotScenario) -> Optional[str]:
        """Create a jury session"""
        payload = {
            "task_ids": scenario.task_ids,
            "members": [m.to_dict() for m in scenario.members],
        }
        
        try:
            async with self.session.post(
                f"{self.api_url}/api/jury/session",
                json=payload,
                timeout=10
            ) as resp:
                if resp.status == 200:
                    data = await resp.json()
                    session_id = data.get("session_id")
                    logger.info(f"✅ Session created: {session_id}")
                    return session_id
                else:
                    logger.error(f"❌ Failed to create session: {resp.status}")
                    return None
        except Exception as e:
            logger.error(f"❌ Error creating session: {e}")
            return None
    
    def _create_commitment(self, vote: bool, nonce: str) -> str:
        """Create SHA256 commitment for a vote"""
        vote_str = "1" if vote else "0"
        commitment = hashlib.sha256(f"{vote_str}|{nonce}".encode()).hexdigest()
        return commitment
    
    async def submit_commitlements(
        self,
        session_id: str,
        scenario: PilotScenario
    ) -> Dict[str, str]:
        """Submit vote commitments from all jury members"""
        nonces = {}
        
        for member in scenario.members:
            agent_id = member.agent_id
            vote = scenario.votes[agent_id]
            nonce = str(uuid.uuid4())
            nonces[agent_id] = nonce
            
            commitment = self._create_commitment(vote, nonce)
            
            payload = {
                "session_id": session_id,
                "member_id": agent_id,
                "type": "commit",
                "commitment": commitment,
            }
            
            try:
                async with self.session.post(
                    f"{self.api_url}/api/jury/vote",
                    json=payload,
                    timeout=10
                ) as resp:
                    if resp.status == 200:
                        logger.info(f"✅ Commitment submitted: {agent_id}")
                    else:
                        logger.error(f"❌ Commit failed for {agent_id}: {resp.status}")
            except Exception as e:
                logger.error(f"❌ Error submitting commitment for {agent_id}: {e}")
        
        return nonces
    
    async def advance_to_reveal(self, session_id: str) -> bool:
        """Transition session from commit to reveal phase"""
        payload = {
            "session_id": session_id,
            "type": "advance",
        }
        
        try:
            async with self.session.post(
                f"{self.api_url}/api/jury/vote",
                json=payload,
                timeout=10
            ) as resp:
                if resp.status == 200:
                    logger.info(f"✅ Transitioned to reveal phase")
                    return True
                else:
                    logger.error(f"❌ Failed to advance to reveal: {resp.status}")
                    return False
        except Exception as e:
            logger.error(f"❌ Error advancing to reveal: {e}")
            return False
    
    async def submit_reveals(
        self,
        session_id: str,
        scenario: PilotScenario,
        nonces: Dict[str, str]
    ) -> bool:
        """Submit vote reveals from all jury members"""
        all_successful = True
        
        for member in scenario.members:
            agent_id = member.agent_id
            vote = scenario.votes[agent_id]
            nonce = nonces[agent_id]
            
            payload = {
                "session_id": session_id,
                "member_id": agent_id,
                "type": "reveal",
                "vote": vote,
                "nonce": nonce,
            }
            
            try:
                async with self.session.post(
                    f"{self.api_url}/api/jury/vote",
                    json=payload,
                    timeout=10
                ) as resp:
                    if resp.status == 200:
                        logger.info(f"✅ Reveal submitted: {agent_id} ({vote})")
                    else:
                        logger.error(f"❌ Reveal failed for {agent_id}: {resp.status}")
                        all_successful = False
            except Exception as e:
                logger.error(f"❌ Error submitting reveal for {agent_id}: {e}")
                all_successful = False
        
        return all_successful
    
    async def aggregate_votes(self, session_id: str) -> Optional[Dict[str, Any]]:
        """Aggregate votes and get final result"""
        payload = {
            "session_id": session_id,
            "type": "aggregate",
        }
        
        try:
            async with self.session.post(
                f"{self.api_url}/api/jury/vote",
                json=payload,
                timeout=10
            ) as resp:
                if resp.status == 200:
                    data = await resp.json()
                    logger.info(f"✅ Votes aggregated")
                    logger.info(f"   YES: {data.get('yes', 0)}")
                    logger.info(f"   NO: {data.get('no', 0)}")
                    logger.info(f"   Result: {data.get('result', 'UNKNOWN')}")
                    return data
                else:
                    logger.error(f"❌ Failed to aggregate: {resp.status}")
                    return None
        except Exception as e:
            logger.error(f"❌ Error aggregating votes: {e}")
            return None
    
    async def get_session_status(self, session_id: str) -> Optional[Dict[str, Any]]:
        """Get current session status and audit trail"""
        try:
            async with self.session.get(
                f"{self.api_url}/api/jury/session/{session_id}",
                timeout=10
            ) as resp:
                if resp.status == 200:
                    data = await resp.json()
                    return data
                else:
                    logger.error(f"❌ Failed to get session: {resp.status}")
                    return None
        except Exception as e:
            logger.error(f"❌ Error getting session: {e}")
            return None
    
    async def run_scenario(self, scenario: PilotScenario) -> Dict[str, Any]:
        """Run a complete pilot test scenario"""
        logger.info("")
        logger.info("=" * 60)
        logger.info(f"SCENARIO: {scenario.name}")
        logger.info("=" * 60)
        
        result = {
            "scenario": scenario.name,
            "status": "FAILED",
            "error": None,
            "session_id": None,
            "expected_result": scenario.expected_result,
            "actual_result": None,
            "votes": {
                "yes": 0,
                "no": 0,
                "total": len(scenario.members),
            },
            "passed": False,
        }
        
        try:
            # Step 1: Create session
            session_id = await self.create_session(scenario)
            if not session_id:
                result["error"] = "Failed to create session"
                self.results.append(result)
                return result
            
            result["session_id"] = session_id
            
            # Step 2: Submit commitments
            logger.info(f"Submitting {len(scenario.members)} commitments...")
            nonces = await self.submit_commitlements(session_id, scenario)
            if len(nonces) != len(scenario.members):
                result["error"] = "Some commitments failed"
                self.results.append(result)
                return result
            
            # Step 3: Wait and advance to reveal
            logger.info("Waiting 5 seconds before advancing to reveal phase...")
            await asyncio.sleep(5)
            
            if not await self.advance_to_reveal(session_id):
                result["error"] = "Failed to advance to reveal"
                self.results.append(result)
                return result
            
            # Step 4: Submit reveals
            logger.info(f"Submitting {len(scenario.members)} reveals...")
            if not await self.submit_reveals(session_id, scenario, nonces):
                result["error"] = "Some reveals failed"
                self.results.append(result)
                return result
            
            # Step 5: Aggregate votes
            logger.info("Aggregating votes...")
            agg_result = await self.aggregate_votes(session_id)
            if not agg_result:
                result["error"] = "Failed to aggregate votes"
                self.results.append(result)
                return result
            
            # Step 6: Verify result
            result["votes"]["yes"] = agg_result.get("yes", 0)
            result["votes"]["no"] = agg_result.get("no", 0)
            actual_result = agg_result.get("result")
            result["actual_result"] = actual_result
            
            # Verify correctness
            if actual_result == scenario.expected_result:
                result["passed"] = True
                result["status"] = "PASSED"
                logger.info(f"✅ VERIFICATION PASSED: Result matches expected ({scenario.expected_result})")
            else:
                result["status"] = "FAILED"
                logger.error(f"❌ VERIFICATION FAILED: Expected {scenario.expected_result}, got {actual_result}")
            
            # Step 7: Get final session status for audit trail
            session_status = await self.get_session_status(session_id)
            if session_status:
                result["audit_trail_events"] = len(session_status.get("audit_trail", []))
                logger.info(f"✅ Audit trail verified: {result['audit_trail_events']} events")
        
        except Exception as e:
            result["error"] = str(e)
            result["status"] = "ERROR"
            logger.error(f"❌ Error running scenario: {e}")
        
        self.results.append(result)
        return result
    
    def print_summary(self):
        """Print test results summary"""
        logger.info("")
        logger.info("=" * 60)
        logger.info("PILOT TEST SUMMARY")
        logger.info("=" * 60)
        
        total = len(self.results)
        passed = sum(1 for r in self.results if r["passed"])
        failed = sum(1 for r in self.results if not r["passed"])
        
        logger.info(f"Total scenarios: {total}")
        logger.info(f"Passed: {passed}")
        logger.info(f"Failed: {failed}")
        logger.info("")
        
        for result in self.results:
            status_symbol = "✅" if result["passed"] else "❌"
            logger.info(f"{status_symbol} {result['scenario']}")
            logger.info(f"   Session: {result['session_id']}")
            logger.info(f"   Expected: {result['expected_result']}")
            logger.info(f"   Actual: {result['actual_result']}")
            logger.info(f"   Votes: {result['votes']['yes']} YES, {result['votes']['no']} NO")
            if result["error"]:
                logger.error(f"   Error: {result['error']}")
            if result.get("audit_trail_events"):
                logger.info(f"   Audit events: {result['audit_trail_events']}")
            logger.info("")
        
        # Overall result
        if failed == 0:
            logger.info("🎉 ALL SCENARIOS PASSED!")
        else:
            logger.error(f"⚠️  {failed} scenario(s) failed")
        
        logger.info("=" * 60)

# ============================================================
# Main Execution
# ============================================================

async def main():
    parser = argparse.ArgumentParser(
        description="Execute pilot test scenarios for jury service"
    )
    parser.add_argument(
        "--api-url",
        default="http://localhost:8000",
        help="API server URL (default: http://localhost:8000)"
    )
    parser.add_argument(
        "--scenario",
        type=int,
        default=0,
        help="Scenario to run (1, 2, or 0 for all)"
    )
    
    args = parser.parse_args()
    
    # Validate scenario
    if args.scenario not in (0, 1, 2):
        logger.error("Invalid scenario. Choose 0 (all), 1, or 2")
        sys.exit(1)
    
    # Determine which scenarios to run
    scenarios_to_run = []
    if args.scenario == 0:
        scenarios_to_run = [SCENARIO_1_INFRASTRUCTURE_UPGRADE, SCENARIO_2_SECURITY_POLICY]
    else:
        scenarios_to_run = [SCENARIOS[args.scenario]]
    
    logger.info(f"Connecting to API: {args.api_url}")
    
    async with PilotExecutor(args.api_url) as executor:
        # Check health first
        if not await executor.check_health():
            logger.error("API is not healthy. Aborting.")
            sys.exit(1)
        
        # Run scenarios
        for idx, scenario in enumerate(scenarios_to_run, 1):
            logger.info(f"\n[{idx}/{len(scenarios_to_run)}] {scenario.name}")
            await executor.run_scenario(scenario)
            
            # Add delay between scenarios
            if idx < len(scenarios_to_run):
                logger.info("Waiting 10 seconds before next scenario...")
                await asyncio.sleep(10)
        
        # Print summary
        executor.print_summary()
        
        # Exit with appropriate code
        failures = sum(1 for r in executor.results if not r["passed"])
        sys.exit(0 if failures == 0 else 1)

if __name__ == "__main__":
    asyncio.run(main())
