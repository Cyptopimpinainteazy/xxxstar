from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
import x3_universal_catalog as catalog


def test_catalog_preserves_every_requirement_and_business_heading():
    source = "# PART 5\n## A. SPOT\n1. Swap.\n- Refund.\n## BUSINESS 1 — Gas\n**Proof:** Real transfer.\n"
    rows = catalog.requirements(source)
    assert [row["name"] for row in rows] == [
        "PART 5", "A. SPOT", "Swap.", "Refund.", "BUSINESS 1 — Gas", "**Proof:** Real transfer."
    ]
    assert rows[2]["category"] == "PART 5 / A. SPOT"
    assert rows[2]["source_line"] == 3
    assert len({row["id"] for row in rows}) == len(rows)


def test_examples_cannot_be_mistaken_for_implemented_features():
    rows = catalog.requirements("## Compiler\n```x3\nswap()\n```\n- Check receipt.\n")
    assert [row["name"] for row in rows] == ["Compiler", "Check receipt."]
    assert all(row["status"] == "BLOCKED" for row in rows)
    assert all(row["audit_state"] == "NOT RUN" for row in rows)
    assert all(row["canonical_feature_ids"] == [] for row in rows)


def test_duplicate_requirements_remain_distinct_and_deterministic():
    source = "## One\n- Refund.\n## Two\n- Refund.\n"
    rows = catalog.requirements(source)
    assert rows == catalog.requirements(source)
    assert rows[1]["id"] != rows[3]["id"]
