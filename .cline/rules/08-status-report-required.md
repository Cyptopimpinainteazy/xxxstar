# Rule: Status Report Required

## Purpose
Every response must communicate actual completion status with an honest status bar. No hiding behind vague language.

## Required Behavior
- Include a status bar with 10-block visual bars for Overall, Code, Tests, Wiring, Docs, and Proof.
- Read `docs/X3_COMPLETION_STATUS.md` for area-level percentages.
- Read `.x3/proof/latest-proof.log` for latest proof results.
- Percentages must be evidence-based, not aspirational.
- If a category is unknown, mark it UNKNOWN and explain how to measure it.

## Forbidden Behavior
- Do NOT fake a high percentage just to look good.
- Do NOT show 100% on anything that lacks end-to-end proof.
- Do NOT omit the status bar from a final response.
- Do NOT report "all green" when proof commands returned errors.

## Proof Required
- Status bar displayed with honest percentages.
- Reference to actual file or log data sources.