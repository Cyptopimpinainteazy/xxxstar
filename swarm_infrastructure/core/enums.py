"""Shared domain enumerations for the X3 AGI substrate.

These enums are referenced by Self-Model, Goal Genome, World Sim, and
Self-Improvement layers.  Keeping them in one place avoids circular imports.
"""

from __future__ import annotations

from enum import Enum


class Domain(str, Enum):
    """Domains in which agents operate and goals exist."""

    CODE = "CODE"
    GOVERNANCE = "GOVERNANCE"
    MARKET = "MARKET"
    INFRASTRUCTURE = "INFRASTRUCTURE"
    NARRATIVE = "NARRATIVE"
    CROSS_DOMAIN = "CROSS_DOMAIN"


class Outcome(str, Enum):
    """Possible outcomes of an agent action."""

    SUCCESS = "SUCCESS"
    FAILURE = "FAILURE"
    PARTIAL = "PARTIAL"
    UNKNOWN = "UNKNOWN"
