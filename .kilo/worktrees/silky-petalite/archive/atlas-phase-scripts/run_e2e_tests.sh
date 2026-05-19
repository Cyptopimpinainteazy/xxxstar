#!/bin/bash

# E2E Test Execution Script for X3-X3-Sphere
# This script runs the end-to-end integration tests

echo "🚀 Starting E2E Integration Tests for X3-X3-Sphere"
echo "=================================================="

# Set the working directory
cd /home/lojak/Desktop/X3-x3-chain

# Function to run a test with timeout
run_test_with_timeout() {
    local test_command="$1"
    local timeout_seconds=60
    
    echo "Running: $test_command"
    echo "With timeout: ${timeout_seconds}s"
    
    # Use timeout to prevent hanging
    if timeout $timeout_seconds bash -c "$test_command"; then
        echo "✅ Test passed: $test_command"
        return 0
    else
        local exit_code=$?
        if [ $exit_code -eq 124 ]; then
            echo "⏰ Test timed out: $test_command"
        else
            echo "❌ Test failed: $test_command (exit code: $exit_code)"
        fi
        return $exit_code
    fi
}

# Test 1: Check if the E2E package is recognized
echo ""
echo "🔍 Test 1: Checking if E2E test package is recognized..."
run_test_with_timeout "cargo test --package e2e_tests --no-run --lib"

# Test 2: Try to run a simple test
echo ""
echo "🔍 Test 2: Running simple E2E tests..."
run_test_with_timeout "cargo test --package e2e_tests simple_test"

# Test 3: Try to run the main infrastructure test
echo ""
echo "🔍 Test 3: Running main infrastructure test..."
run_test_with_timeout "cargo test --package e2e_tests test_e2e_infrastructure_setup"

# Test 4: Try to run all E2E tests
echo ""
echo "🔍 Test 4: Running all E2E tests..."
run_test_with_timeout "cargo test --package e2e_tests"

echo ""
echo "📊 Test execution completed!"
echo "Check the output above for results."

# Show test results summary
echo ""
echo "📋 Test Summary:"
echo "- If tests compiled successfully: E2E infrastructure is working"
echo "- If tests ran successfully: Integration logic is functional"
echo "- If tests failed: Issues need to be debugged and fixed"

exit 0
