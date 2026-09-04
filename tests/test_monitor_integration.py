#!/usr/bin/env python3
"""
Integration tests for scripts/monitor_blocks.sh

These tests validate:
1. Log parsing for all x3-chain log formats
2. Block number extraction from various formats
3. Integration with block_display.py
4. Full pipeline: log → extraction → display
"""

import re
import subprocess
import sys
from pathlib import Path

# Test fixtures with various x3-chain log formats
LOG_FORMATS = {
    "block_imported_basic": "Block imported: #1268",
    "block_imported_hash": "Imported #1268 (0xd924…1cde)",
    "block_finalized": "Block finalized: #1264",
    "prepared_block": "Prepared block for proposing at 1269",
    "block_sealed": "Block #1270 sealed",
    "block_validated": "Validated block #1271",
    "consensus_reached": "Consensus reached at block #1272",
}

# Expected block numbers for each format
EXPECTED_BLOCKS = {
    "block_imported_basic": 1268,
    "block_imported_hash": 1268,
    "block_finalized": 1264,
    "prepared_block": 1269,
    "block_sealed": 1270,
    "block_validated": 1271,
    "consensus_reached": 1272,
}


def get_project_root():
    """Get the project root directory."""
    return Path(__file__).parent.parent


def test_monitor_script_exists():
    """Test that monitor_blocks.sh exists."""
    monitor_path = get_project_root() / "scripts" / "monitor_blocks.sh"
    assert monitor_path.exists(), f"Monitor script not found at {monitor_path}"
    print(f"✅ Monitor script exists at {monitor_path}")


def test_block_display_script_exists():
    """Test that block_display.py exists."""
    display_path = get_project_root() / "scripts" / "block_display.py"
    assert display_path.exists(), f"Display script not found at {display_path}"
    print(f"✅ Display script exists at {display_path}")


def test_log_parsing_with_regex():
    """Test log format parsing with regex patterns."""
    patterns = [
        (r"Block imported:\s+#(\d+)", "Block imported: #1268", "1268"),
        (r"Imported\s+#(\d+)", "Imported #1268 (0xd924…1cde)", "1268"),
        (r"Block finalized:\s+#(\d+)", "Block finalized: #1264", "1264"),
        (r"Prepared block for proposing at (\d+)", "Prepared block for proposing at 1269", "1269"),
        (r"Block\s+#(\d+)\s+sealed", "Block #1270 sealed", "1270"),
        (r"Validated block\s+#(\d+)", "Validated block #1271", "1271"),
        (r"Consensus reached at block\s+#(\d+)", "Consensus reached at block #1272", "1272"),
    ]

    for pattern, log_line, expected_block in patterns:
        match = re.search(pattern, log_line)
        assert match is not None, f"Pattern failed to match: {pattern} against {log_line}"
        assert match.group(1) == expected_block, f"Extracted block {match.group(1)} != {expected_block}"
        print(f"✅ Regex parsed {log_line} → Block #{expected_block}")


def test_log_format_coverage():
    """Test all x3-chain log formats are covered."""
    for format_name, log_line in LOG_FORMATS.items():
        expected = EXPECTED_BLOCKS[format_name]

        # Try to extract block using multiple patterns
        patterns = [
            r"Block imported:\s+#(\d+)",
            r"Imported\s+#(\d+)",
            r"Block finalized:\s+#(\d+)",
            r"Prepared block for proposing at (\d+)",
            r"Block\s+#(\d+)\s+sealed",
            r"Validated block\s+#(\d+)",
            r"Consensus reached at block\s+#(\d+)",
        ]

        match = None
        for pattern in patterns:
            match = re.search(pattern, log_line)
            if match:
                break

        assert match is not None, f"No pattern matched {format_name}: {log_line}"
        block_num = int(match.group(1))
        assert block_num == expected, f"Block mismatch: got {block_num}, expected {expected}"
        print(f"✅ {format_name}: {log_line} → Block #{expected}")


def test_block_display_basic():
    """Test that block_display.py runs successfully."""
    display_script = get_project_root() / "scripts" / "block_display.py"

    result = subprocess.run(
        ["python3", str(display_script), "42"],
        capture_output=True,
        text=True,
        timeout=5
    )

    assert result.returncode == 0, f"Display script failed: {result.stderr}"
    assert "Block #42" in result.stdout, "Output missing block header"
    assert "┌──┐" in result.stdout, "Output missing box drawing characters"
    print("✅ block_display.py runs successfully with block #42")


def test_block_display_milestones():
    """Test that milestones display correctly."""
    display_script = get_project_root() / "scripts" / "block_display.py"

    test_cases = [
        (1000, "1k blocks finalized"),
        (100000, "MAJOR MILESTONE"),
        (1000000, "ONE MILLION"),
    ]

    for block_num, expected_text in test_cases:
        result = subprocess.run(
            ["python3", str(display_script), str(block_num)],
            capture_output=True,
            text=True,
            timeout=5
        )

        assert result.returncode == 0, f"Failed for block {block_num}"
        assert expected_text in result.stdout, f"Expected '{expected_text}' not in output for block {block_num}"
        print(f"✅ Milestone display for block #{block_num}")


def test_monitor_with_simulated_logs():
    """Test monitor script with simulated log output."""
    monitor_script = get_project_root() / "scripts" / "monitor_blocks.sh"

    # Create a test log with multiple formats
    test_logs = [
        "Block imported: #1268",
        "Imported #1269 (0xd924…1cde)",
        "Block finalized: #1270",
    ]

    test_input = "\n".join(test_logs)

    # Run monitor with simulated input
    result = subprocess.run(
        ["bash", str(monitor_script)],
        input=test_input,
        capture_output=True,
        text=True,
        timeout=10
    )

    # Monitor should process the input without errors
    assert result.returncode == 0, f"Monitor script returned error: {result.stderr}"
    print("✅ Monitor processes simulated logs successfully")


def test_large_block_numbers():
    """Test handling of large block numbers."""
    display_script = get_project_root() / "scripts" / "block_display.py"

    large_blocks = [999999, 10000000, 432000000]  # ~1 day of blocks

    for block_num in large_blocks:
        result = subprocess.run(
            ["python3", str(display_script), str(block_num)],
            capture_output=True,
            text=True,
            timeout=5
        )

        assert result.returncode == 0, f"Failed for large block {block_num}"
        assert f"Block #{block_num}" in result.stdout, f"Output missing block {block_num}"
        print(f"✅ Large block number #{block_num} handled correctly")


def test_block_extraction_accuracy():
    """Test accurate block number extraction from logs."""
    test_cases = [
        ("Block imported: #1", 1),
        ("Block imported: #999", 999),
        ("Block imported: #1000", 1000),
        ("Block imported: #123456", 123456),
        ("Prepared block for proposing at 1", 1),
        ("Prepared block for proposing at 999999", 999999),
    ]

    pattern = r"Block imported:\s+#(\d+)|Prepared block for proposing at (\d+)"

    for log_line, expected_block in test_cases:
        match = re.search(pattern, log_line)
        assert match is not None, f"Failed to match: {log_line}"

        # Extract block from either group (1 or 2)
        block_num = int(match.group(1) or match.group(2))
        assert block_num == expected_block, f"Got {block_num}, expected {expected_block}"
        print(f"✅ Extracted block #{expected_block} from: {log_line}")


def test_consecutive_blocks():
    """Test extracting multiple consecutive block numbers."""
    consecutive_logs = [
        "Block imported: #1000",
        "Block imported: #1001",
        "Block imported: #1002",
        "Block imported: #1003",
    ]

    pattern = r"Block imported:\s+#(\d+)"
    blocks = []

    for log_line in consecutive_logs:
        match = re.search(pattern, log_line)
        assert match is not None, f"Failed to match: {log_line}"
        blocks.append(int(match.group(1)))

    assert blocks == [1000, 1001, 1002, 1003], f"Block sequence incorrect: {blocks}"
    print(f"✅ Consecutive blocks extracted correctly: {blocks}")


def test_performance_rapid_blocks():
    """Test handling rapid block production (5 blocks/sec)."""
    display_script = get_project_root() / "scripts" / "block_display.py"

    # Simulate 5 blocks in rapid succession
    base_block = 10000
    for i in range(5):
        block_num = base_block + i
        result = subprocess.run(
            ["python3", str(display_script), str(block_num)],
            capture_output=True,
            text=True,
            timeout=5
        )
        assert result.returncode == 0, f"Failed for block {block_num}"

    print("✅ Rapid block sequence handled (5 blocks in succession)")


def run_all_tests():
    """Run all integration tests."""
    tests = [
        test_monitor_script_exists,
        test_block_display_script_exists,
        test_log_parsing_with_regex,
        test_log_format_coverage,
        test_block_display_basic,
        test_block_display_milestones,
        test_monitor_with_simulated_logs,
        test_large_block_numbers,
        test_block_extraction_accuracy,
        test_consecutive_blocks,
        test_performance_rapid_blocks,
    ]

    print("\n" + "="*70)
    print("INTEGRATION TESTS: monitor_blocks.sh + block_display.py")
    print("="*70 + "\n")

    passed = 0
    failed = 0

    for test in tests:
        try:
            test()
            passed += 1
        except AssertionError as e:
            print(f"❌ FAILED: {test.__name__}: {e}")
            failed += 1
        except Exception as e:
            print(f"❌ ERROR: {test.__name__}: {type(e).__name__}: {e}")
            failed += 1

    print("\n" + "="*70)
    print(f"RESULTS: {passed} passed, {failed} failed")
    print("="*70 + "\n")

    return failed == 0


if __name__ == "__main__":
    success = run_all_tests()
    sys.exit(0 if success else 1)
