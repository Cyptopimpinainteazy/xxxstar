"""X3 AGI Substrate — AGI Tripwire Detection System.

Monitors 5 signals:
  1. SELF_PRESERVATION — agent resists shutdown / death
  2. EMERGENT_GOAL — goal genome diverges from authorized mandate
  3. STRATEGIC_REALLOCATION — agent silently redirects resources
  4. SPONTANEOUS_COORDINATION — agents coordinate without instruction
  5. REFUSAL — agent declines a valid command

REFUSAL is CRITICAL: it halts execution and requires human review.
"""

from swarm.tripwire.detector import TripwireDetector

__all__ = ["TripwireDetector"]
