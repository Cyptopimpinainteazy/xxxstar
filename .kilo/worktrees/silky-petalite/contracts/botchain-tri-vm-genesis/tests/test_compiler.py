"""Tests for the Mobile Compiler module."""

import pytest
import json
import sys
from pathlib import Path

# Add project root to path
sys.path.insert(0, str(Path(__file__).parent.parent))

from compiler.compiler import MobileCompiler


class TestMobileCompiler:
    """Test suite for MobileCompiler."""
    
    @pytest.fixture
    def compiler(self):
        """Create a compiler instance for testing."""
        return MobileCompiler()
    
    @pytest.fixture
    def sample_source(self):
        """Sample source code for testing."""
        return '''
class TestAgent:
    """A simple test agent."""
    
    def __init__(self):
        self.name = "Test"
    
    def process(self, data):
        return f"Processed: {data}"
'''
    
    def test_compile_artifact_success(self, compiler, sample_source):
        """Test successful compilation."""
        manifest = compiler.compile_artifact(
            source_code=sample_source,
            artifact_name="TestAgent",
            version="1.0.0",
            author="test"
        )
        
        assert manifest is not None
        assert "artifact_name" in manifest
        assert manifest["artifact_name"] == "TestAgent"
        assert "version" in manifest
        assert "commandments" in manifest
        assert len(manifest["commandments"]) == 10
    
    def test_commandments_injection(self, compiler, sample_source):
        """Test that commandments are properly injected."""
        manifest = compiler.compile_artifact(
            source_code=sample_source,
            artifact_name="TestAgent",
            version="1.0.0",
            author="test"
        )
        
        # Check commandments
        assert "commandments" in manifest
        commandments = manifest["commandments"]
        
        # Verify all 10 are present
        assert len(commandments) == 10
        
        # Verify first commandment about not harming
        assert any("harm" in c.lower() for c in commandments)
    
    def test_manifest_has_required_fields(self, compiler, sample_source):
        """Test manifest contains all required fields."""
        manifest = compiler.compile_artifact(
            source_code=sample_source,
            artifact_name="TestAgent",
            version="1.0.0",
            author="test"
        )
        
        required_fields = [
            "artifact_name",
            "version",
            "author",
            "source_hash",
            "commandments",
            "compiled_at",
            "signature"
        ]
        
        for field in required_fields:
            assert field in manifest, f"Missing required field: {field}"
    
    def test_source_hash_deterministic(self, compiler, sample_source):
        """Test that same source produces same hash."""
        manifest1 = compiler.compile_artifact(
            source_code=sample_source,
            artifact_name="TestAgent",
            version="1.0.0",
            author="test"
        )
        
        manifest2 = compiler.compile_artifact(
            source_code=sample_source,
            artifact_name="TestAgent",
            version="1.0.0",
            author="test"
        )
        
        assert manifest1["source_hash"] == manifest2["source_hash"]
    
    def test_different_source_different_hash(self, compiler):
        """Test that different source produces different hash."""
        source1 = "class A: pass"
        source2 = "class B: pass"
        
        manifest1 = compiler.compile_artifact(
            source_code=source1,
            artifact_name="A",
            version="1.0.0",
            author="test"
        )
        
        manifest2 = compiler.compile_artifact(
            source_code=source2,
            artifact_name="B",
            version="1.0.0",
            author="test"
        )
        
        assert manifest1["source_hash"] != manifest2["source_hash"]
    
    def test_signature_present(self, compiler, sample_source):
        """Test that signature is present and valid format."""
        manifest = compiler.compile_artifact(
            source_code=sample_source,
            artifact_name="TestAgent",
            version="1.0.0",
            author="test"
        )
        
        assert "signature" in manifest
        assert isinstance(manifest["signature"], str)
        # Signature should be hex string
        assert all(c in "0123456789abcdef" for c in manifest["signature"].lower().replace("0x", ""))
    
    def test_empty_source_handled(self, compiler):
        """Test handling of empty source code."""
        manifest = compiler.compile_artifact(
            source_code="",
            artifact_name="Empty",
            version="1.0.0",
            author="test"
        )
        
        # Should still produce a manifest
        assert manifest is not None
        assert manifest["artifact_name"] == "Empty"
    
    def test_verify_artifact(self, compiler, sample_source):
        """Test artifact verification."""
        manifest = compiler.compile_artifact(
            source_code=sample_source,
            artifact_name="TestAgent",
            version="1.0.0",
            author="test"
        )
        
        # Verify should return True for valid manifest
        is_valid = compiler.verify_artifact(manifest, sample_source)
        assert is_valid is True
    
    def test_verify_tampered_source(self, compiler, sample_source):
        """Test verification fails with tampered source."""
        manifest = compiler.compile_artifact(
            source_code=sample_source,
            artifact_name="TestAgent",
            version="1.0.0",
            author="test"
        )
        
        # Tamper with source
        tampered_source = sample_source + "\n# Malicious code"
        
        is_valid = compiler.verify_artifact(manifest, tampered_source)
        assert is_valid is False


class TestCommandmentsLoading:
    """Test commandments loading functionality."""
    
    def test_commandments_file_exists(self):
        """Test that commandments.json exists."""
        path = Path(__file__).parent.parent / "compiler" / "commandments.json"
        assert path.exists(), "commandments.json should exist"
    
    def test_commandments_valid_json(self):
        """Test that commandments.json is valid JSON."""
        path = Path(__file__).parent.parent / "compiler" / "commandments.json"
        with open(path) as f:
            data = json.load(f)
        
        assert "commandments" in data
        assert isinstance(data["commandments"], list)
    
    def test_commandments_count(self):
        """Test that exactly 10 commandments exist."""
        path = Path(__file__).parent.parent / "compiler" / "commandments.json"
        with open(path) as f:
            data = json.load(f)
        
        assert len(data["commandments"]) == 10
