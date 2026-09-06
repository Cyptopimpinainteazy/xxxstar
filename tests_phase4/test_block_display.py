#!/usr/bin/env python3
"""
Unit tests for scripts/block_display.py

Comprehensive test suite covering:
- Milestone detection (is_milestone)
- Milestone level determination (get_milestone_level)
- Color cycling (get_color)
- Block number display rendering
- Edge cases and boundary conditions
- CLI behavior
"""

import os
import sys

import pytest

# Add scripts directory to path for imports
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', 'scripts'))

# Import functions to test
from block_display import (
    BOLD,
    COLORS,
    RESET,
    display_big_milestone,
    display_block_number,
    display_explosive_milestone,
    display_tiny_milestone,
    get_color,
    get_milestone_level,
    is_milestone,
)

# ============================================================================
# MILESTONE DETECTION TESTS
# ============================================================================

class TestMilestoneDetection:
    """Test is_milestone() function for identifying milestone blocks."""

    def test_is_milestone_1k_blocks(self):
        """Test 1k milestone detection."""
        assert is_milestone(1000)
        assert is_milestone(2000)
        assert is_milestone(3000)
        assert is_milestone(10000)

    def test_is_milestone_100k_blocks(self):
        """Test 100k milestone detection."""
        assert is_milestone(100000)
        assert is_milestone(200000)
        assert is_milestone(500000)
        assert is_milestone(900000)

    def test_is_milestone_1m_blocks(self):
        """Test 1M milestone detection."""
        assert is_milestone(1000000)
        assert is_milestone(2000000)
        assert is_milestone(10000000)
        assert is_milestone(100000000)

    def test_non_milestone_blocks(self):
        """Test non-milestone blocks return False."""
        assert not is_milestone(1)
        assert not is_milestone(42)
        assert not is_milestone(999)
        assert not is_milestone(1001)
        assert not is_milestone(5787)
        assert not is_milestone(99999)
        assert not is_milestone(100001)
        assert not is_milestone(999999)

    def test_milestone_boundary_conditions(self):
        """Test blocks just before and after milestones."""
        assert not is_milestone(999)
        assert is_milestone(1000)
        assert not is_milestone(1001)

        assert not is_milestone(99999)
        assert is_milestone(100000)
        assert not is_milestone(100001)

        assert not is_milestone(999999)
        assert is_milestone(1000000)
        assert not is_milestone(1000001)


# ============================================================================
# MILESTONE LEVEL DETERMINATION TESTS
# ============================================================================

class TestMilestoneLevel:
    """Test get_milestone_level() function."""

    def test_tiny_milestone_level(self):
        """Test 'tiny' milestone level (1k blocks)."""
        assert get_milestone_level(1000) == "tiny"
        assert get_milestone_level(2000) == "tiny"
        assert get_milestone_level(50000) == "tiny"

    def test_big_milestone_level(self):
        """Test 'big' milestone level (100k blocks)."""
        assert get_milestone_level(100000) == "big"
        assert get_milestone_level(200000) == "big"
        assert get_milestone_level(500000) == "big"

    def test_explosive_milestone_level(self):
        """Test 'explosive' milestone level (1M blocks)."""
        assert get_milestone_level(1000000) == "explosive"
        assert get_milestone_level(2000000) == "explosive"
        assert get_milestone_level(100000000) == "explosive"

    def test_normal_level_non_milestones(self):
        """Test 'normal' level for non-milestone blocks."""
        assert get_milestone_level(1) == "normal"
        assert get_milestone_level(42) == "normal"
        assert get_milestone_level(999) == "normal"
        assert get_milestone_level(5787) == "normal"

    def test_milestone_precedence(self):
        """Test that larger milestones take precedence."""
        # 1M is both 1k and 100k and 1M milestone, should return "explosive"
        assert get_milestone_level(1000000) == "explosive"
        # 100k is both 1k and 100k milestone, should return "big"
        assert get_milestone_level(100000) == "big"


# ============================================================================
# COLOR CYCLING TESTS
# ============================================================================

class TestColorCycling:
    """Test get_color() function for ANSI color cycling."""

    def test_color_cycling_within_range(self):
        """Test color cycling for indices within COLORS length."""
        for i in range(len(COLORS)):
            assert get_color(i) == COLORS[i]

    def test_color_cycling_wraps_around(self):
        """Test that color cycling wraps around after reaching max."""
        # After COLORS length, should wrap to start
        assert get_color(len(COLORS)) == COLORS[0]
        assert get_color(len(COLORS) + 1) == COLORS[1]

    def test_color_cycling_large_indices(self):
        """Test color cycling with very large indices."""
        large_idx = 1000
        expected_idx = large_idx % len(COLORS)
        assert get_color(large_idx) == COLORS[expected_idx]

    def test_color_is_ansi_code(self):
        """Test that returned colors are valid ANSI codes."""
        for i in range(10):
            color = get_color(i)
            assert "\033[" in color or color == ""  # ANSI escape sequences


# ============================================================================
# BLOCK DISPLAY OUTPUT TESTS
# ============================================================================

class TestBlockDisplay:
    """Test display_block_number() output formatting."""

    def test_single_digit_block(self, capsys):
        """Test display of single-digit block (1-9)."""
        display_block_number(1)
        captured = capsys.readouterr()
        assert "Block #1" in captured.out
        assert "┌──┐" in captured.out
        assert "│1 │" in captured.out
        assert "└──┘" in captured.out

    def test_two_digit_block(self, capsys):
        """Test display of two-digit block (42)."""
        display_block_number(42)
        captured = capsys.readouterr()
        assert "Block #42" in captured.out
        # Should have 2 sets of boxes (one for each digit)
        assert captured.out.count("┌──┐") >= 2
        assert "│4 │" in captured.out
        assert "│2 │" in captured.out

    def test_four_digit_block(self, capsys):
        """Test display of four-digit block (5787)."""
        display_block_number(5787)
        captured = capsys.readouterr()
        assert "Block #5787" in captured.out
        # Should have 4 sets of boxes
        assert captured.out.count("┌──┐") >= 4
        assert "│5 │" in captured.out
        assert "│7 │" in captured.out
        assert "│8 │" in captured.out

    def test_large_block_number(self, capsys):
        """Test display of large block number."""
        display_block_number(999999)
        captured = capsys.readouterr()
        assert "Block #999999" in captured.out
        # Should have 6 sets of boxes
        assert captured.out.count("┌──┐") >= 6
        assert "│9 │" in captured.out

    def test_display_contains_ansi_codes(self, capsys):
        """Test that display output contains ANSI color codes."""
        display_block_number(42)
        captured = capsys.readouterr()
        # Should contain ANSI escape codes
        assert "\033[" in captured.out
        # Should contain reset code
        assert RESET in captured.out

    def test_display_box_structure(self, capsys):
        """Test that display maintains proper box structure."""
        display_block_number(42)
        captured = capsys.readouterr()
        lines = captured.out.split('\n')
        # Find the box lines (should have multiple lines with boxes)
        box_lines = [line for line in lines if "┌──┐" in line or "│" in line or "└──┘" in line]
        assert len(box_lines) >= 3  # At least top, middle, bottom


# ============================================================================
# MILESTONE DISPLAY TESTS
# ============================================================================

class TestTinyMilestoneDisplay:
    """Test display_tiny_milestone() output."""

    def test_tiny_milestone_output(self, capsys):
        """Test that tiny milestone produces expected output."""
        display_tiny_milestone(1000)
        captured = capsys.readouterr()
        assert "1k blocks finalized" in captured.out
        assert "✦" in captured.out

    def test_tiny_milestone_various_values(self, capsys):
        """Test tiny milestone display for various k values."""
        display_tiny_milestone(5000)
        captured = capsys.readouterr()
        assert "5k blocks finalized" in captured.out

        # Capture new output separately
        display_tiny_milestone(100000)
        captured = capsys.readouterr()
        assert "100k blocks finalized" in captured.out


class TestBigMilestoneDisplay:
    """Test display_big_milestone() output."""

    def test_big_milestone_output(self, capsys):
        """Test that big milestone produces expected output."""
        display_big_milestone(100000)
        captured = capsys.readouterr()
        assert "MAJOR MILESTONE" in captured.out
        assert "100,000" in captured.out
        assert "◆" in captured.out
        assert "╔" in captured.out  # Box drawing character

    def test_big_milestone_various_values(self, capsys):
        """Test big milestone display for various 100k values."""
        display_big_milestone(200000)
        captured = capsys.readouterr()
        assert "200,000" in captured.out
        assert "2 × 100,000" in captured.out


class TestExplosiveMilestoneDisplay:
    """Test display_explosive_milestone() output."""

    def test_one_million_milestone(self, capsys):
        """Test 1 million block display."""
        display_explosive_milestone(1000000)
        captured = capsys.readouterr()
        assert "ONE MILLION BLOCKS" in captured.out
        assert "1,000,000" in captured.out
        assert "🎉" in captured.out

    def test_ten_million_milestone(self, capsys):
        """Test 10 million block display."""
        display_explosive_milestone(10000000)
        captured = capsys.readouterr()
        assert "TEN MILLION BLOCKS" in captured.out
        assert "10,000,000" in captured.out
        assert "◆" in captured.out

    def test_hundred_million_milestone(self, capsys):
        """Test 100 million block display."""
        display_explosive_milestone(100000000)
        captured = capsys.readouterr()
        assert "ONE HUNDRED MILLION BLOCKS" in captured.out
        assert "100,000,000" in captured.out

    def test_generic_million_milestone(self, capsys):
        """Test generic million milestone (not 1M, 10M, or 100M)."""
        display_explosive_milestone(3000000)
        captured = capsys.readouterr()
        assert "3 MILLION BLOCKS" in captured.out or "MILLION" in captured.out
        assert "3,000,000" in captured.out


# ============================================================================
# INTEGRATION TESTS
# ============================================================================

class TestIntegration:
    """Integration tests combining multiple functions."""

    def test_milestone_1k_display(self, capsys):
        """Integration: Detect and display 1k milestone."""
        assert is_milestone(1000)
        assert get_milestone_level(1000) == "tiny"
        display_block_number(1000)
        captured = capsys.readouterr()
        assert "1k blocks finalized" in captured.out
        assert "✦" in captured.out

    def test_milestone_100k_display(self, capsys):
        """Integration: Detect and display 100k milestone."""
        assert is_milestone(100000)
        assert get_milestone_level(100000) == "big"
        display_block_number(100000)
        captured = capsys.readouterr()
        assert "MAJOR MILESTONE" in captured.out

    def test_milestone_1m_display(self, capsys):
        """Integration: Detect and display 1M milestone."""
        assert is_milestone(1000000)
        assert get_milestone_level(1000000) == "explosive"
        display_block_number(1000000)
        captured = capsys.readouterr()
        assert "ONE MILLION BLOCKS" in captured.out

    def test_non_milestone_display(self, capsys):
        """Integration: Display non-milestone block with boxes."""
        assert not is_milestone(5787)
        display_block_number(5787)
        captured = capsys.readouterr()
        assert "Block #5787" in captured.out
        assert "┌──┐" in captured.out  # Should show boxes, not milestone

    def test_multiple_blocks_sequential(self, capsys):
        """Integration: Display multiple different blocks."""
        display_block_number(42)
        display_block_number(1000)
        display_block_number(5787)
        captured = capsys.readouterr()
        assert "Block #42" in captured.out
        assert "1k blocks finalized" in captured.out
        assert "Block #5787" in captured.out


# ============================================================================
# EDGE CASE TESTS
# ============================================================================

class TestEdgeCases:
    """Test edge cases and boundary conditions."""

    def test_block_zero_not_milestone(self):
        """Test that block 0 is not a milestone."""
        assert not is_milestone(0)

    def test_very_large_block_number(self, capsys):
        """Test display of very large block number."""
        large_block = 999999999
        display_block_number(large_block)
        captured = capsys.readouterr()
        assert "Block #999999999" in captured.out
        # Should have 9 digit boxes
        assert captured.out.count("│") >= 9

    def test_block_nine_vs_ten(self, capsys):
        """Test transition from single to double digit."""
        display_block_number(9)
        captured = capsys.readouterr()
        assert "Block #9" in captured.out
        assert captured.out.count("┌──┐") >= 1

    def test_color_consistency_across_colors(self):
        """Test that all colors are non-empty strings."""
        for i in range(10):
            color = get_color(i)
            assert isinstance(color, str)
            assert len(color) > 0


# ============================================================================
# OUTPUT FORMAT TESTS
# ============================================================================

class TestOutputFormat:
    """Test consistency and format of output."""

    def test_output_has_header_and_boxes(self, capsys):
        """Test that output includes both header and boxes."""
        display_block_number(42)
        captured = capsys.readouterr()
        # Must have header
        assert "Block #" in captured.out
        # Must have box drawing characters
        assert "┌──┐" in captured.out

    def test_output_ends_with_newline(self, capsys):
        """Test that output ends with newline."""
        display_block_number(42)
        captured = capsys.readouterr()
        assert captured.out.endswith('\n')

    def test_milestone_output_structure(self, capsys):
        """Test milestone output has proper structure."""
        display_block_number(1000)
        captured = capsys.readouterr()
        # Should have bold codes and colors
        assert BOLD in captured.out or "\033[" in captured.out

    def test_no_error_output(self, capsys):
        """Test that normal display produces no stderr."""
        display_block_number(42)
        captured = capsys.readouterr()
        assert captured.err == ""


# ============================================================================
# FUNCTION PARAMETER TESTS
# ============================================================================

class TestFunctionParameters:
    """Test function behavior with various parameters."""

    def test_is_milestone_with_negative(self):
        """Test is_milestone with negative numbers."""
        # Negative blocks are not valid milestones
        assert not is_milestone(-1000)
        assert not is_milestone(-100000)
        assert not is_milestone(-42)

    def test_get_color_with_zero(self):
        """Test get_color with index 0."""
        assert get_color(0) == COLORS[0]

    def test_get_milestone_level_checks_precedence(self):
        """Test that milestone level checks in correct order."""
        # 2M should be explosive (1M), not tiny (2k) or big (200k)
        assert get_milestone_level(2000000) == "explosive"
        # But 2k alone should be tiny
        result_2k = get_milestone_level(2000)
        assert result_2k == "tiny"


# ============================================================================
# RUN TESTS
# ============================================================================

if __name__ == "__main__":
    pytest.main([__file__, "-v", "--tb=short"])
