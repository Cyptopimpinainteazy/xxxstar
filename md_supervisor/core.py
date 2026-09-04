"""
md_supervisor orchestration engine — the state machine that ties ingestion,
deduplication, arbitration, patching, quality gates, and commit into a
deterministic pipeline.

Pipeline: Ingest → Deduplicate → Arbitrate → Gate → Patch → Commit → Report
"""
import asyncio
import logging
from pathlib import Path
from typing import List, Optional, Dict, Tuple

from md_supervisor.schema import ChangeRequest, AuditEntry
from md_supervisor.ingestion import parse_text, parse_chat_file
from md_supervisor.dedupe import deduplicate, is_conflict, prioritize_conflicts
from md_supervisor.arbitration import get_default_arbitration
from md_supervisor.gates import GatePipeline
from md_supervisor.patcher import apply_change, rollback, preview_diff

logger = logging.getLogger("md_supervisor.core")


class MdSupervisor:
    """
    Main supervisor orchestrator. Runs the full ingestion-to-commit pipeline.
    """
    
    def __init__(self, dry_run: bool = False):
        self.dry_run = dry_run
        self.arbitration = get_default_arbitration()
        self.gate_pipeline = GatePipeline()
        self.audit_log: List[AuditEntry] = []
        self.changes: List[ChangeRequest] = []
    
    def ingest_text(self, text: str) -> List[ChangeRequest]:
        """Parse text and add to pending changes."""
        requests = parse_text(text)
        self.changes.extend(requests)
        logger.info(f"Ingested {len(requests)} changes from text")
        return requests
    
    def ingest_file(self, path: Path) -> List[ChangeRequest]:
        """Parse a file and add to pending changes."""
        requests = parse_chat_file(path)
        self.changes.extend(requests)
        logger.info(f"Ingested {len(requests)} changes from {path}")
        return requests
    
    def run_pipeline(self) -> Dict:
        """
        Run the full pipeline on all pending changes.
        Returns a detailed report of everything that happened.
        """
        report = {
            "ingested": len(self.changes),
            "deduplicated": 0,
            "approved": 0,
            "rejected": 0,
            "gated": 0,
            "applied": 0,
            "failed": 0,
            "details": [],
        }
        
        if not self.changes:
            report["message"] = "No pending changes"
            return report
        
        # Phase 1: Deduplicate
        unique = deduplicate(self.changes)
        report["deduplicated"] = len(self.changes) - len(unique)
        self.changes = unique
        
        # Phase 2: Conflict detection
        conflicts = []
        for i in range(len(self.changes)):
            for j in range(i + 1, len(self.changes)):
                if is_conflict(self.changes[i], self.changes[j]):
                    conflicts.append([self.changes[i], self.changes[j]])
        
        if conflicts:
            resolved = prioritize_conflicts(conflicts)
            # Replace conflicted changes with winners
            self.changes = [c for c in self.changes if c in resolved]
            report["details"].append(f"Resolved {len(conflicts)} conflict groups")
        
        # Phase 3: Arbitrate + Gate + Apply (per change)
        for req in self.changes:
            detail = {"id": req.id, "files": [f.path for f in req.files], "intent": str(req.intent)}
            
            # Arbitration
            approved, votes, score = self.arbitration.vote(req)
            detail["arbitration"] = {"approved": approved, "votes": votes, "score": score}
            
            if not approved:
                detail["status"] = "rejected"
                report["rejected"] += 1
                report["details"].append(detail)
                continue
            
            # Quality gates
            self.gate_pipeline.run_all(req)
            gates_passed = self.gate_pipeline.all_passed()
            detail["gates"] = self.gate_pipeline.report()
            
            if not gates_passed:
                detail["status"] = "gated"
                report["gated"] += 1
                report["details"].append(detail)
                continue
            
            # Apply patch
            try:
                audit = apply_change(req, dry_run=self.dry_run)
                self.audit_log.extend(audit)
                detail["status"] = "applied" if not self.dry_run else "dry_run"
                detail["audit"] = [str(a) for a in audit]
                report["applied"] += 1
            except Exception as e:
                detail["status"] = "failed"
                detail["error"] = str(e)
                report["failed"] += 1
            
            report["details"].append(detail)
        
        report["message"] = (
            f"Pipeline complete: {report['applied']} applied, "
            f"{report['rejected']} rejected, {report['gated']} gated, "
            f"{report['failed']} failed, {report['deduplicated']} deduplicated"
        )
        
        return report
    
    def rollback_change(self, change_id: str) -> bool:
        """Rollback a specific change by ID."""
        for req in self.changes:
            if req.id == change_id:
                try:
                    audit = rollback(req)
                    self.audit_log.extend(audit)
                    logger.info(f"Rolled back {change_id}")
                    return True
                except Exception as e:
                    logger.error(f"Rollback failed for {change_id}: {e}")
                    return False
        return False
    
    def clear(self):
        """Clear all pending changes (not rolled back changes)."""
        self.changes.clear()
    
    def get_status(self) -> Dict:
        """Get current supervisor status."""
        return {
            "pending_changes": len(self.changes),
            "audit_log_entries": len(self.audit_log),
            "dry_run": self.dry_run,
        }