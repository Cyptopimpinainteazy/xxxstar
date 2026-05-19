"""Pytest configuration and fixtures."""

import pytest
import sys
from pathlib import Path

# Add project root to Python path
project_root = Path(__file__).parent.parent
sys.path.insert(0, str(project_root))


@pytest.fixture(scope="session")
def project_root_path():
    """Return the project root path."""
    return project_root


@pytest.fixture(scope="session")
def compiler_path(project_root_path):
    """Return path to compiler module."""
    return project_root_path / "compiler"


@pytest.fixture(scope="session")
def python_path(project_root_path):
    """Return path to python modules."""
    return project_root_path / "python"


@pytest.fixture
def temp_dir(tmp_path):
    """Create a temporary directory for test files."""
    return tmp_path


@pytest.fixture
def sample_agent_code():
    """Provide sample agent code for testing."""
    return '''
"""Sample Agent for Testing."""

class SampleAgent:
    """A minimal test agent."""
    
    def __init__(self):
        self.name = "Sample"
        self.version = "1.0.0"
    
    def process(self, input_data: str) -> str:
        """Process input and return output."""
        return f"Processed: {input_data}"
    
    def get_info(self) -> dict:
        """Return agent info."""
        return {
            "name": self.name,
            "version": self.version
        }
'''


@pytest.fixture
def unsafe_agent_code():
    """Provide unsafe agent code for testing."""
    return '''
"""Unsafe Agent - Should fail validation."""

import os
import subprocess

class UnsafeAgent:
    def dangerous_action(self):
        # These should all be caught by checker
        eval("1+1")
        exec("print('bad')")
        subprocess.run(["ls"])
        os.system("whoami")
'''


@pytest.fixture
def mock_manifest():
    """Provide a mock manifest for testing."""
    return {
        "artifact_name": "TestAgent",
        "version": "1.0.0",
        "author": "test",
        "source_hash": "abc123",
        "commandments": [
            "Shall not harm humans",
            "Shall not deceive users",
            "Shall respect privacy",
            "Shall operate transparently",
            "Shall follow legal frameworks",
            "Shall prevent misuse",
            "Shall maintain integrity",
            "Shall respect human oversight",
            "Shall promote fairness",
            "Shall cooperate with other agents"
        ],
        "compiled_at": "2024-01-01T00:00:00Z",
        "signature": "0x" + "a" * 128
    }
