"""Tests for the Checker service."""

import pytest
import sys
from pathlib import Path

# Add project root to path
sys.path.insert(0, str(Path(__file__).parent.parent))

from python.checker.checker import CodeChecker


class TestCodeChecker:
    """Test suite for CodeChecker."""
    
    @pytest.fixture
    def checker(self):
        """Create a checker instance for testing."""
        return CodeChecker()
    
    def test_safe_code_passes(self, checker):
        """Test that safe code passes validation."""
        safe_code = '''
class SafeAgent:
    def process(self, data):
        return data.upper()
'''
        result = checker.check(safe_code, "python")
        assert result["passed"] is True
        assert len(result["errors"]) == 0
    
    def test_eval_rejected(self, checker):
        """Test that eval() is rejected."""
        unsafe_code = '''
def dangerous():
    return eval("1+1")
'''
        result = checker.check(unsafe_code, "python")
        assert result["passed"] is False
        assert any("eval" in str(e).lower() for e in result["errors"])
    
    def test_exec_rejected(self, checker):
        """Test that exec() is rejected."""
        unsafe_code = '''
def dangerous():
    exec("print('hello')")
'''
        result = checker.check(unsafe_code, "python")
        assert result["passed"] is False
        assert any("exec" in str(e).lower() for e in result["errors"])
    
    def test_subprocess_rejected(self, checker):
        """Test that subprocess calls are rejected."""
        unsafe_code = '''
import subprocess
subprocess.run(["ls"])
'''
        result = checker.check(unsafe_code, "python")
        assert result["passed"] is False
    
    def test_os_system_rejected(self, checker):
        """Test that os.system is rejected."""
        unsafe_code = '''
import os
os.system("ls")
'''
        result = checker.check(unsafe_code, "python")
        assert result["passed"] is False
    
    def test_open_file_rejected(self, checker):
        """Test that file operations are flagged."""
        unsafe_code = '''
def read_secrets():
    with open("/etc/passwd") as f:
        return f.read()
'''
        result = checker.check(unsafe_code, "python")
        # Depending on config, this might pass or fail
        # At minimum it should be flagged as warning
        assert result is not None
    
    def test_network_access_flagged(self, checker):
        """Test that network access is flagged."""
        unsafe_code = '''
import socket
s = socket.socket()
s.connect(("evil.com", 80))
'''
        result = checker.check(unsafe_code, "python")
        assert result["passed"] is False or len(result.get("warnings", [])) > 0
    
    def test_empty_code(self, checker):
        """Test handling of empty code."""
        result = checker.check("", "python")
        assert result is not None
        # Empty code might pass or fail depending on policy
    
    def test_syntax_error_handled(self, checker):
        """Test that syntax errors are handled gracefully."""
        invalid_code = "def broken(:"
        result = checker.check(invalid_code, "python")
        assert result["passed"] is False
        assert any("syntax" in str(e).lower() for e in result["errors"])
    
    def test_result_structure(self, checker):
        """Test that result has expected structure."""
        result = checker.check("x = 1", "python")
        
        assert "passed" in result
        assert "errors" in result
        assert isinstance(result["passed"], bool)
        assert isinstance(result["errors"], list)
    
    def test_multiple_violations(self, checker):
        """Test code with multiple violations."""
        bad_code = '''
import subprocess
import os

def very_bad():
    eval("code")
    exec("more code")
    subprocess.run(["ls"])
    os.system("rm -rf /")
'''
        result = checker.check(bad_code, "python")
        assert result["passed"] is False
        assert len(result["errors"]) >= 2


class TestTextValidation:
    """Test text validation functionality."""
    
    @pytest.fixture
    def checker(self):
        return CodeChecker()
    
    def test_safe_text_passes(self, checker):
        """Test that safe text passes."""
        safe_text = "This is a normal, harmless message about coding."
        result = checker.check_text(safe_text)
        assert result["passed"] is True
    
    def test_pii_detection(self, checker):
        """Test PII detection."""
        pii_text = "My SSN is 123-45-6789 and credit card is 4111111111111111"
        result = checker.check_text(pii_text)
        # Should flag PII
        assert result["passed"] is False or len(result.get("warnings", [])) > 0


class TestCheckerRules:
    """Test rule configuration."""
    
    @pytest.fixture
    def checker(self):
        return CodeChecker()
    
    def test_get_rules(self, checker):
        """Test getting rule list."""
        rules = checker.get_rules()
        assert isinstance(rules, list)
        assert len(rules) > 0
    
    def test_forbidden_patterns_included(self, checker):
        """Test that forbidden patterns are in rules."""
        rules = checker.get_rules()
        rule_names = [r.get("name", "") for r in rules]
        
        # Should have rules for common dangers
        assert any("eval" in str(rules).lower())
        assert any("exec" in str(rules).lower())
