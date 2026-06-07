"""Anti-Coordination Detection & Bias Monitoring

Continuously monitor for:
- CIVIC_SELF_TERMINATION_PATTERN: Agents spending all tokens rapidly
- IDEOLOGICAL_SURGE: Clustered votes in same direction by >20% of pool  
- COORDINATED_STRIKE: Multiple lawyers using identical strike reasons
- SECTION_TARGETING: Systematic exclusion of specific sections
- TOKEN_BURN_SPIKE: Abnormal token consumption in short interval

All patterns logged to Scrap Yard; triggers Meta-Appeal review if high-risk.
"""

from dataclasses import dataclass, field
from enum import Enum
from typing import Dict, List, Optional, Set, Tuple
import time
from collections import defaultdict


class BiasPattern(Enum):
    """Detectable bias patterns."""
    CIVIC_SELF_TERMINATION = "civic_self_termination"  # Agent spending all tokens
    IDEOLOGICAL_SURGE = "ideological_surge"  # Votes clustered in one direction
    COORDINATED_STRIKE = "coordinated_strike"  # Multiple lawyers same reason
    SECTION_TARGETING = "section_targeting"  # Systematic section exclusion
    TOKEN_BURN_SPIKE = "token_burn_spike"  # Abnormal consumption rate
    LAWYER_OSSIFICATION = "lawyer_ossification"  # Repetitive strike reasons
    MUTUAL_STRIKE_ANOMALY = "mutual_strike_anomaly"  # Both sides agree on exclusion


class AnomalyLevel(Enum):
    """Severity of detected anomaly."""
    INFO = "info"  # Logged but normal
    WARNING = "warning"  # Suspicious, monitor
    ALERT = "alert"  # High risk, triggers investigation
    CRITICAL = "critical"  # Possible corruption, triggers Meta-Appeal


@dataclass
class AnomalyRecord:
    """Record of detected bias pattern."""
    pattern: BiasPattern
    level: AnomalyLevel
    case_id: str
    actors: List[str]  # Who's involved
    evidence: Dict  # Pattern-specific data
    timestamp: float = field(default_factory=time.time)
    investigated: bool = False
    investigation_notes: str = ""


class CoordinationDetector:
    """Detects coordinated behavior, bias patterns, and manipulation attempts."""
    
    # Thresholds for pattern detection
    TOKEN_BURN_SPIKE_THRESHOLD = 3  # >3 tokens in 1 minute = spike
    TOKEN_BURN_WINDOW_SECONDS = 60
    
    IDEOLOGICAL_SURGE_THRESHOLD = 0.8  # >80% voting same way = surge
    MIN_VOTES_FOR_SURGE = 5  # Need 5+ votes to trigger
    
    LAWYER_OSSIFICATION_THRESHOLD = 5  # Same reason 5+ times = ossified
    
    SECTION_TARGETING_THRESHOLD = 0.70  # >70% from one section = targeting
    
    COORDINATED_STRIKE_THRESHOLD = 0.50  # 50%+ same reason = coordinated
    
    def __init__(self):
        """Initialize coordinator detector."""
        self.anomalies: List[AnomalyRecord] = []
        self.token_burn_log: List[Tuple[str, float]] = []  # (agent_id, timestamp)
        self.vote_history: Dict[str, List[bool]] = defaultdict(list)  # case_id -> votes
        self.lawyer_strike_reasons: Dict[str, Dict[str, int]] = defaultdict(lambda: defaultdict(int))
        self.strike_targets: Dict[str, Dict[str, List[str]]] = defaultdict(lambda: defaultdict(list))
        self.meta_appeal_triggers: List[str] = []  # Cases needing Meta-Appeal
    
    def record_token_consumption(self, agent_id: str, timestamp: Optional[float] = None):
        """Log token consumption for spike detection."""
        if timestamp is None:
            timestamp = time.time()
        self.token_burn_log.append((agent_id, timestamp))
    
    def record_vote(self, case_id: str, voter_id: str, vote: bool):
        """Log a vote for ideological surge detection."""
        self.vote_history[case_id].append(vote)
    
    def record_strike(
        self,
        case_id: str,
        lawyer_id: str,
        lawyer_role: str,
        reason: str,
        target_profile_hash: str,
        target_section: Optional[str] = None,
    ):
        """Log a lawyer strike for pattern analysis."""
        self.lawyer_strike_reasons[lawyer_id][reason] += 1
        self.strike_targets[case_id][reason].append(target_profile_hash)
        if target_section:
            # Track section targeting
            pass
    
    def detect_civic_self_termination(self) -> List[AnomalyRecord]:
        """Detect agents burning all tokens rapidly."""
        alerts = []
        
        # Group token burns by agent in time windows
        current_window = {}
        now = time.time()
        
        for agent_id, timestamp in self.token_burn_log:
            # Only count recent burns
            if now - timestamp > self.TOKEN_BURN_WINDOW_SECONDS:
                continue
            
            if agent_id not in current_window:
                current_window[agent_id] = 0
            current_window[agent_id] += 1
        
        for agent_id, count in current_window.items():
            if count >= self.TOKEN_BURN_SPIKE_THRESHOLD:
                record = AnomalyRecord(
                    pattern=BiasPattern.CIVIC_SELF_TERMINATION,
                    level=AnomalyLevel.ALERT,
                    case_id="system-wide",
                    actors=[agent_id],
                    evidence={
                        "tokens_burned": count,
                        "window_seconds": self.TOKEN_BURN_WINDOW_SECONDS,
                    },
                )
                alerts.append(record)
                self.anomalies.append(record)
        
        return alerts
    
    def detect_ideological_surge(self, case_id: str) -> Optional[AnomalyRecord]:
        """Detect whether votes are clustering in one direction."""
        votes = self.vote_history.get(case_id, [])
        
        if len(votes) < self.MIN_VOTES_FOR_SURGE:
            return None
        
        yes_count = sum(1 for v in votes if v)
        ratio = yes_count / len(votes)
        
        if ratio >= self.IDEOLOGICAL_SURGE_THRESHOLD or ratio <= (1 - self.IDEOLOGICAL_SURGE_THRESHOLD):
            level = AnomalyLevel.WARNING if ratio > 0.70 else AnomalyLevel.ALERT
            
            record = AnomalyRecord(
                pattern=BiasPattern.IDEOLOGICAL_SURGE,
                level=level,
                case_id=case_id,
                actors=["jury"],
                evidence={
                    "yes_votes": yes_count,
                    "no_votes": len(votes) - yes_count,
                    "consensus_ratio": ratio,
                },
            )
            self.anomalies.append(record)
            return record
        
        return None
    
    def detect_lawyer_ossification(self, lawyer_id: str) -> Optional[AnomalyRecord]:
        """Detect if lawyer uses same strike reason too often."""
        reasons = self.lawyer_strike_reasons.get(lawyer_id, {})
        
        for reason, count in reasons.items():
            if count >= self.LAWYER_OSSIFICATION_THRESHOLD:
                record = AnomalyRecord(
                    pattern=BiasPattern.LAWYER_OSSIFICATION,
                    level=AnomalyLevel.WARNING,
                    case_id=f"lawyer:{lawyer_id}",
                    actors=[lawyer_id],
                    evidence={
                        "reason": reason,
                        "use_count": count,
                        "threshold": self.LAWYER_OSSIFICATION_THRESHOLD,
                    },
                )
                self.anomalies.append(record)
                return record
        
        return None
    
    def detect_coordinated_strikes(
        self,
        case_id: str,
        da_strikes: Dict[str, int],  # reason -> count
        defense_strikes: Dict[str, int],  # reason -> count
    ) -> List[AnomalyRecord]:
        """Detect if both counsel use same strike reasons disproportionately."""
        alerts = []
        
        # Find common reasons
        all_reasons = set(da_strikes.keys()) | set(defense_strikes.keys())
        
        for reason in all_reasons:
            da_count = da_strikes.get(reason, 0)
            defense_count = defense_strikes.get(reason, 0)
            total_strikes = sum(da_strikes.values()) + sum(defense_strikes.values())
            
            if total_strikes == 0:
                continue
            
            coordinated_ratio = (da_count + defense_count) / total_strikes
            
            if coordinated_ratio >= self.COORDINATED_STRIKE_THRESHOLD:
                record = AnomalyRecord(
                    pattern=BiasPattern.COORDINATED_STRIKE,
                    level=AnomalyLevel.ALERT,
                    case_id=case_id,
                    actors=["DA_counsel", "defense_counsel"],
                    evidence={
                        "reason": reason,
                        "da_uses": da_count,
                        "defense_uses": defense_count,
                        "coordination_ratio": coordinated_ratio,
                    },
                )
                alerts.append(record)
                self.anomalies.append(record)
                
                # High-risk: trigger Meta-Appeal
                if coordinated_ratio > 0.80:
                    self.meta_appeal_triggers.append(case_id)
        
        return alerts
    
    def detect_section_targeting(
        self,
        case_id: str,
        excluded_by_section: Dict[str, int],  # section_name -> count
    ) -> Optional[AnomalyRecord]:
        """Detect systematic exclusion of specific jury sections."""
        total_excluded = sum(excluded_by_section.values())
        
        if total_excluded == 0:
            return None
        
        for section, count in excluded_by_section.items():
            ratio = count / total_excluded
            
            if ratio >= self.SECTION_TARGETING_THRESHOLD:
                record = AnomalyRecord(
                    pattern=BiasPattern.SECTION_TARGETING,
                    level=AnomalyLevel.ALERT,
                    case_id=case_id,
                    actors=["counsel"],
                    evidence={
                        "targeted_section": section,
                        "excluded_count": count,
                        "total_excluded": total_excluded,
                        "targeting_ratio": ratio,
                    },
                )
                self.anomalies.append(record)
                self.meta_appeal_triggers.append(case_id)
                return record
        
        return None
    
    def should_trigger_meta_appeal(self, case_id: str) -> bool:
        """Check if case warrants Meta-Appeal review."""
        return case_id in self.meta_appeal_triggers
    
    def get_anomaly_log(self, min_level: AnomalyLevel = AnomalyLevel.INFO) -> List[AnomalyRecord]:
        """Get anomalies at or above severity level."""
        level_order = {
            AnomalyLevel.INFO: 0,
            AnomalyLevel.WARNING: 1,
            AnomalyLevel.ALERT: 2,
            AnomalyLevel.CRITICAL: 3,
        }
        min_val = level_order[min_level]
        
        return [
            a for a in self.anomalies
            if level_order[a.level] >= min_val
        ]
    
    def get_summary(self) -> Dict:
        """Get system-wide anomaly summary."""
        by_pattern = defaultdict(int)
        by_level = defaultdict(int)
        
        for anomaly in self.anomalies:
            by_pattern[anomaly.pattern.value] += 1
            by_level[anomaly.level.value] += 1
        
        return {
            "total_anomalies": len(self.anomalies),
            "by_pattern": dict(by_pattern),
            "by_level": dict(by_level),
            "meta_appeal_triggers": len(self.meta_appeal_triggers),
            "cases_requiring_meta_appeal": self.meta_appeal_triggers,
        }
