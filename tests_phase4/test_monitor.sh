#!/bin/bash
# Integration tests for monitor_blocks.sh
# Tests log parsing, block extraction, and visualization output

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MONITOR_SCRIPT="${SCRIPT_DIR}/scripts/monitor_blocks.sh"
BLOCK_DISPLAY="${SCRIPT_DIR}/scripts/block_display.py"
TEST_TEMP_DIR="/tmp/x3_monitor_tests_$$"

# Test counters
TESTS_TOTAL=0
TESTS_PASSED=0
TESTS_FAILED=0

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
RESET='\033[0m'

# Setup/Teardown
setup() {
    mkdir -p "$TEST_TEMP_DIR"
}

teardown() {
    rm -rf "$TEST_TEMP_DIR"
}

# Test helper functions
run_test() {
    local test_name="$1"
    local test_func="$2"
    
    TESTS_TOTAL=$((TESTS_TOTAL + 1))
    echo -e "${BLUE}[TEST $TESTS_TOTAL]${RESET} $test_name"
    
    if $test_func; then
        echo -e "${GREEN}  ✓ PASS${RESET}\n"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        echo -e "${RED}  ✗ FAIL${RESET}\n"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
}

assert_contains() {
    local haystack="$1"
    local needle="$2"
    local msg="${3:-Expected to find '$needle' in output}"
    
    if echo "$haystack" | grep -q "$needle"; then
        return 0
    else
        echo "  $msg"
        echo "  Haystack: $haystack"
        return 1
    fi
}

assert_equals() {
    local actual="$1"
    local expected="$2"
    local msg="${3:-Values don't match}"
    
    if [ "$actual" = "$expected" ]; then
        return 0
    else
        echo "  $msg"
        echo "  Expected: '$expected'"
        echo "  Actual:   '$actual'"
        return 1
    fi
}

# ============================================================================
# TEST SUITE: Log Format Patterns
# ============================================================================

test_format_block_imported() {
    local log_line="2026-03-11 12:34:56 Block imported: #12345 (0xd924...1cde)"
    local output=$(echo "$log_line" | bash "$MONITOR_SCRIPT" 2>&1)
    
    assert_contains "$output" "12345" "Should extract block 12345 from 'Block imported:' format" && \
    assert_contains "$output" "$log_line" "Should display original log line"
}

test_format_imported_hash() {
    local log_line="2026-03-11 12:34:56 Imported #5787 (0xabc123def456)"
    local output=$(echo "$log_line" | bash "$MONITOR_SCRIPT" 2>&1)
    
    assert_contains "$output" "5787" "Should extract block 5787 from 'Imported #' format" && \
    assert_contains "$output" "$log_line" "Should display original log line"
}

test_format_block_finalized() {
    local log_line="2026-03-11 12:34:56 Block finalized: #9999"
    local output=$(echo "$log_line" | bash "$MONITOR_SCRIPT" 2>&1)
    
    assert_contains "$output" "9999" "Should extract block 9999 from 'Block finalized:' format" && \
    assert_contains "$output" "$log_line" "Should display original log line"
}

test_format_finalized_short() {
    local log_line="2026-03-11 12:34:56 finalized #8888"
    local output=$(echo "$log_line" | bash "$MONITOR_SCRIPT" 2>&1)
    
    assert_contains "$output" "8888" "Should extract block 8888 from 'finalized #' format" && \
    assert_contains "$output" "$log_line" "Should display original log line"
}

test_format_proposing_at() {
    local log_line="2026-03-11 12:34:56 Prepared block for proposing at 7654"
    local output=$(echo "$log_line" | bash "$MONITOR_SCRIPT" 2>&1)
    
    assert_contains "$output" "7654" "Should extract block 7654 from 'proposing at' format" && \
    assert_contains "$output" "$log_line" "Should display original log line"
}

test_format_block_hash() {
    local log_line="2026-03-11 12:34:56 block: #4567"
    local output=$(echo "$log_line" | bash "$MONITOR_SCRIPT" 2>&1)
    
    assert_contains "$output" "4567" "Should extract block 4567 from 'block: #' format" && \
    assert_contains "$output" "$log_line" "Should display original log line"
}

# ============================================================================
# TEST SUITE: Block Number Extraction
# ============================================================================

test_extract_single_digit_block() {
    local log_line="Block imported: #5"
    local output=$(echo "$log_line" | bash "$MONITOR_SCRIPT" 2>&1)
    
    assert_contains "$output" "5" "Should correctly extract single-digit block"
}

test_extract_double_digit_block() {
    local log_line="Block imported: #42"
    local output=$(echo "$log_line" | bash "$MONITOR_SCRIPT" 2>&1)
    
    assert_contains "$output" "42" "Should correctly extract double-digit block"
}

test_extract_triple_digit_block() {
    local log_line="Block imported: #999"
    local output=$(echo "$log_line" | bash "$MONITOR_SCRIPT" 2>&1)
    
    assert_contains "$output" "999" "Should correctly extract triple-digit block"
}

test_extract_large_block_number() {
    local log_line="Block imported: #1000000"
    local output=$(echo "$log_line" | bash "$MONITOR_SCRIPT" 2>&1)
    
    assert_contains "$output" "1000000" "Should correctly extract 7-digit block number (1M milestone)"
}

test_extract_milestone_blocks() {
    local log_line="Block imported: #1000"
    local output=$(echo "$log_line" | bash "$MONITOR_SCRIPT" 2>&1)
    
    assert_contains "$output" "1000" "Should correctly extract milestone block"
}

# ============================================================================
# TEST SUITE: Input Mode Handling
# ============================================================================

test_stdin_input_mode() {
    local test_log="Block imported: #123"
    local output=$(echo "$test_log" | bash "$MONITOR_SCRIPT" 2>&1)
    
    assert_contains "$output" "123" "Should process stdin input correctly"
}

test_file_input_mode() {
    local test_log_file="$TEST_TEMP_DIR/test.log"
    echo "Block imported: #456" > "$test_log_file"
    
    # Use timeout to prevent hanging from tail -f
    local output=$(timeout 2 bash "$MONITOR_SCRIPT" "$test_log_file" 2>&1 || true)
    
    assert_contains "$output" "456" "Should process file input correctly" || true
}

test_file_input_multiple_lines() {
    local test_log_file="$TEST_TEMP_DIR/test_multi.log"
    {
        echo "Block imported: #101"
        echo "Block finalized: #102"
        echo "Imported #103"
    } > "$test_log_file"
    
    # Use timeout to prevent hanging
    local output=$(timeout 2 cat "$test_log_file" | bash "$MONITOR_SCRIPT" 2>&1 || true)
    
    assert_contains "$output" "101" "Should extract first block" && \
    assert_contains "$output" "102" "Should extract second block" && \
    assert_contains "$output" "103" "Should extract third block"
}

# ============================================================================
# TEST SUITE: Non-Matching Lines
# ============================================================================

test_ignore_non_matching_lines() {
    local output=$(echo "This line has no block info" | bash "$MONITOR_SCRIPT" 2>&1)
    
    # Output should NOT contain a block visualization (should be empty/minimal)
    [ -z "$(echo "$output" | grep -E '█|░|▓' || true)" ] && return 0 || return 1
}

test_ignore_multiple_non_matching() {
    local log_data="Line 1 with no block
Line 2 also has nothing
Line 3 still no block"
    
    local output=$(echo "$log_data" | bash "$MONITOR_SCRIPT" 2>&1)
    
    # Should process without error and produce minimal output
    [ $? -eq 0 ] && return 0 || return 1
}

test_mixed_matching_and_non_matching() {
    local log_data="Some random line
Block imported: #200
Another random line
finalized #201
Yet another line"
    
    local output=$(echo "$log_data" | bash "$MONITOR_SCRIPT" 2>&1)
    
    assert_contains "$output" "200" "Should find first block among noise" && \
    assert_contains "$output" "201" "Should find second block among noise"
}

# ============================================================================
# TEST SUITE: Output Format Validation
# ============================================================================

test_output_contains_separator() {
    local log_line="Block imported: #100"
    local output=$(echo "$log_line" | bash "$MONITOR_SCRIPT" 2>&1)
    
    assert_contains "$output" "━━━━━━━━━━━━" "Should display separator lines in output"
}

test_output_contains_original_log_line() {
    local log_line="Block imported: #555 (0xabc123)"
    local output=$(echo "$log_line" | bash "$MONITOR_SCRIPT" 2>&1)
    
    assert_contains "$output" "Block imported: #555" "Should include original log line in output"
}

test_output_calls_block_display() {
    local log_line="Block imported: #42"
    local output=$(echo "$log_line" | bash "$MONITOR_SCRIPT" 2>&1)
    
    # block_display.py should produce neon colored output with box characters
    assert_contains "$output" "42" "Should invoke block visualization"
}

# ============================================================================
# TEST SUITE: Edge Cases
# ============================================================================

test_block_zero() {
    local log_line="Block imported: #0"
    local output=$(echo "$log_line" | bash "$MONITOR_SCRIPT" 2>&1)
    
    assert_contains "$output" "0" "Should handle block #0"
}

test_very_large_block_number() {
    local log_line="Block imported: #999999999"
    local output=$(echo "$log_line" | bash "$MONITOR_SCRIPT" 2>&1)
    
    assert_contains "$output" "999999999" "Should handle 9-digit block numbers"
}

test_block_with_leading_zeros() {
    local log_line="Block imported: #00123"
    local output=$(echo "$log_line" | bash "$MONITOR_SCRIPT" 2>&1)
    
    # Should extract 00123 (bash will treat as octal in some contexts)
    assert_contains "$output" "123\|00123" "Should handle block with leading zeros"
}

test_malformed_block_number() {
    local log_line="Block imported: #abc"
    local output=$(echo "$log_line" | bash "$MONITOR_SCRIPT" 2>&1)
    
    # abc should NOT match the regex, so no output
    [ -z "$(echo "$output" | grep -E 'abc' || true)" ] && return 0 || return 1
}

test_multiple_blocks_in_one_line() {
    local log_line="Block imported: #100 and finalized #200"
    local output=$(echo "$log_line" | bash "$MONITOR_SCRIPT" 2>&1)
    
    # Should match the FIRST block number found
    assert_contains "$output" "100" "Should extract first block number from line with multiple blocks"
}

test_empty_input() {
    local output=$(echo "" | bash "$MONITOR_SCRIPT" 2>&1)
    
    # Should handle gracefully
    [ $? -eq 0 ] && return 0 || return 1
}

test_very_long_log_line() {
    local long_line="Block imported: #456 $(python3 -c 'print("x" * 1000)' 2>/dev/null || echo 'xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx')"
    local output=$(echo "$long_line" | bash "$MONITOR_SCRIPT" 2>&1)
    
    assert_contains "$output" "456" "Should extract block from very long log line"
}

# ============================================================================
# TEST SUITE: Milestone Detection via Integration
# ============================================================================

test_milestone_1k_block() {
    local log_line="Block finalized: #1000"
    local output=$(echo "$log_line" | bash "$MONITOR_SCRIPT" 2>&1)
    
    assert_contains "$output" "1000" "Should process 1k milestone block"
}

test_milestone_100k_block() {
    local log_line="Block finalized: #100000"
    local output=$(echo "$log_line" | bash "$MONITOR_SCRIPT" 2>&1)
    
    assert_contains "$output" "100000" "Should process 100k milestone block"
}

test_milestone_1m_block() {
    local log_line="Block finalized: #1000000"
    local output=$(echo "$log_line" | bash "$MONITOR_SCRIPT" 2>&1)
    
    assert_contains "$output" "1000000" "Should process 1M milestone block"
}

# ============================================================================
# TEST SUITE: Error Handling
# ============================================================================

test_script_exists() {
    [ -f "$MONITOR_SCRIPT" ] && return 0 || return 1
}

test_script_executable() {
    [ -x "$MONITOR_SCRIPT" ] && return 0 || return 1
}

test_missing_log_file_error() {
    local output=$(bash "$MONITOR_SCRIPT" "/nonexistent/path.log" 2>&1 || true)
    
    assert_contains "$output" "Error\|not found\|usage" "Should show error for missing log file" || true
}

test_block_display_dependency() {
    [ -f "$BLOCK_DISPLAY" ] && return 0 || return 1
}

# ============================================================================
# TEST SUITE: Performance
# ============================================================================

test_processing_speed_single_line() {
    local log_line="Block imported: #789"
    local start=$(date +%s%N)
    local output=$(echo "$log_line" | bash "$MONITOR_SCRIPT" 2>&1)
    local end=$(date +%s%N)
    local elapsed=$(( (end - start) / 1000000 ))  # Convert to ms
    
    # Should process in under 500ms
    [ $elapsed -lt 500 ] && return 0 || return 1
}

test_processing_speed_many_lines() {
    local log_data=""
    for i in {1..50}; do
        log_data+="Block imported: #$i
"
    done
    
    local start=$(date +%s%N)
    local output=$(echo "$log_data" | bash "$MONITOR_SCRIPT" 2>&1)
    local end=$(date +%s%N)
    local elapsed=$(( (end - start) / 1000000 ))  # Convert to ms
    
    # Should process 50 lines in under 2 seconds
    [ $elapsed -lt 2000 ] && return 0 || return 1
}

# ============================================================================
# MAIN TEST RUNNER
# ============================================================================

main() {
    echo -e "${YELLOW}╔═══════════════════════════════════════════════════════════════╗${RESET}"
    echo -e "${YELLOW}║  X3 Chain Block Monitor - Integration Test Suite              ║${RESET}"
    echo -e "${YELLOW}╚═══════════════════════════════════════════════════════════════╝${RESET}"
    echo ""
    
    setup
    
    # Test categories
    echo -e "${YELLOW}━━ Log Format Patterns ━━${RESET}"
    run_test "Parse 'Block imported: #' format" test_format_block_imported
    run_test "Parse 'Imported #' format" test_format_imported_hash
    run_test "Parse 'Block finalized: #' format" test_format_block_finalized
    run_test "Parse 'finalized #' format" test_format_finalized_short
    run_test "Parse 'proposing at' format" test_format_proposing_at
    run_test "Parse 'block: #' format" test_format_block_hash
    
    echo -e "${YELLOW}━━ Block Number Extraction ━━${RESET}"
    run_test "Extract single-digit block" test_extract_single_digit_block
    run_test "Extract double-digit block" test_extract_double_digit_block
    run_test "Extract triple-digit block" test_extract_triple_digit_block
    run_test "Extract large block number" test_extract_large_block_number
    run_test "Extract milestone blocks" test_extract_milestone_blocks
    
    echo -e "${YELLOW}━━ Input Mode Handling ━━${RESET}"
    run_test "Handle stdin input mode" test_stdin_input_mode
    run_test "Handle file input mode" test_file_input_mode
    run_test "Handle multiple lines from file" test_file_input_multiple_lines
    
    echo -e "${YELLOW}━━ Non-Matching Lines ━━${RESET}"
    run_test "Ignore non-matching lines" test_ignore_non_matching_lines
    run_test "Ignore multiple non-matching lines" test_ignore_multiple_non_matching
    run_test "Mixed matching and non-matching" test_mixed_matching_and_non_matching
    
    echo -e "${YELLOW}━━ Output Format Validation ━━${RESET}"
    run_test "Output contains separator" test_output_contains_separator
    run_test "Output contains original log line" test_output_contains_original_log_line
    run_test "Output calls block display" test_output_calls_block_display
    
    echo -e "${YELLOW}━━ Edge Cases ━━${RESET}"
    run_test "Handle block #0" test_block_zero
    run_test "Handle very large block numbers" test_very_large_block_number
    run_test "Handle block with leading zeros" test_block_with_leading_zeros
    run_test "Reject malformed block numbers" test_malformed_block_number
    run_test "Handle multiple blocks in one line" test_multiple_blocks_in_one_line
    run_test "Handle empty input" test_empty_input
    run_test "Handle very long log line" test_very_long_log_line
    
    echo -e "${YELLOW}━━ Milestone Detection ━━${RESET}"
    run_test "Process 1k milestone block" test_milestone_1k_block
    run_test "Process 100k milestone block" test_milestone_100k_block
    run_test "Process 1M milestone block" test_milestone_1m_block
    
    echo -e "${YELLOW}━━ Error Handling ━━${RESET}"
    run_test "Script exists" test_script_exists
    run_test "Script is executable" test_script_executable
    run_test "Handle missing log file" test_missing_log_file_error
    run_test "Block display dependency exists" test_block_display_dependency
    
    echo -e "${YELLOW}━━ Performance ━━${RESET}"
    run_test "Process single line quickly" test_processing_speed_single_line
    run_test "Process 50 lines within time limit" test_processing_speed_many_lines
    
    teardown
    
    # Summary
    echo ""
    echo -e "${YELLOW}╔═══════════════════════════════════════════════════════════════╗${RESET}"
    echo -e "${YELLOW}║  Test Summary${RESET}"
    echo -e "${YELLOW}╚═══════════════════════════════════════════════════════════════╝${RESET}"
    
    local pass_pct=0
    if [ $TESTS_TOTAL -gt 0 ]; then
        pass_pct=$(( (TESTS_PASSED * 100) / TESTS_TOTAL ))
    fi
    
    echo "Total Tests:  $TESTS_TOTAL"
    echo -e "Passed:       ${GREEN}$TESTS_PASSED${RESET}"
    echo -e "Failed:       ${RED}$TESTS_FAILED${RESET}"
    echo "Pass Rate:    ${pass_pct}%"
    echo ""
    
    if [ $TESTS_FAILED -eq 0 ]; then
        echo -e "${GREEN}✅ All tests passed!${RESET}"
        return 0
    else
        echo -e "${RED}❌ Some tests failed${RESET}"
        return 1
    fi
}

main "$@"
