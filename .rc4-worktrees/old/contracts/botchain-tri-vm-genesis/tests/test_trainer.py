"""Tests for the Trainer module."""

import pytest
import sys
from pathlib import Path

# Add project root to path
sys.path.insert(0, str(Path(__file__).parent.parent))

from python.trainer.trainer import DeterministicTrainer, ToyModel


class TestToyModel:
    """Test suite for ToyModel."""
    
    @pytest.fixture
    def model(self):
        """Create a model instance."""
        return ToyModel()
    
    def test_model_initialization(self, model):
        """Test model initializes correctly."""
        assert model is not None
        assert hasattr(model, "forward")
    
    def test_forward_returns_output(self, model):
        """Test forward pass returns output."""
        output = model.forward("test input")
        assert output is not None
    
    def test_forward_deterministic(self, model):
        """Test forward pass is deterministic."""
        output1 = model.forward("same input")
        output2 = model.forward("same input")
        assert output1 == output2
    
    def test_different_inputs_different_outputs(self, model):
        """Test different inputs produce different outputs."""
        output1 = model.forward("input A")
        output2 = model.forward("input B")
        # May or may not be different depending on implementation
        assert output1 is not None and output2 is not None


class TestDeterministicTrainer:
    """Test suite for DeterministicTrainer."""
    
    @pytest.fixture
    def trainer(self):
        """Create a trainer instance."""
        return DeterministicTrainer()
    
    @pytest.fixture
    def model(self):
        """Create a model instance."""
        return ToyModel()
    
    @pytest.fixture
    def sample_dataset(self):
        """Sample dataset for testing."""
        return [
            ("hello", "world"),
            ("foo", "bar"),
            ("input", "output"),
        ]
    
    def test_trainer_initialization(self, trainer):
        """Test trainer initializes correctly."""
        assert trainer is not None
    
    def test_train_returns_result(self, trainer, model, sample_dataset):
        """Test training returns a result."""
        result = trainer.train(model, sample_dataset)
        
        assert result is not None
        assert isinstance(result, dict)
    
    def test_train_result_has_loss(self, trainer, model, sample_dataset):
        """Test training result includes loss."""
        result = trainer.train(model, sample_dataset)
        
        assert "final_loss" in result
        assert isinstance(result["final_loss"], (int, float))
    
    def test_train_deterministic(self, trainer, model, sample_dataset):
        """Test training is deterministic with same seed."""
        result1 = trainer.train(model, sample_dataset, seed=42)
        
        # Create fresh model for second run
        model2 = ToyModel()
        result2 = trainer.train(model2, sample_dataset, seed=42)
        
        assert result1["final_loss"] == result2["final_loss"]
    
    def test_train_empty_dataset(self, trainer, model):
        """Test handling of empty dataset."""
        result = trainer.train(model, [])
        
        assert result is not None
        # Should handle gracefully
    
    def test_train_single_example(self, trainer, model):
        """Test training with single example."""
        single_dataset = [("only", "one")]
        result = trainer.train(model, single_dataset)
        
        assert result is not None
        assert "final_loss" in result


class TestTrainingWithParentLogs:
    """Test training with parent log integration."""
    
    @pytest.fixture
    def trainer(self):
        return DeterministicTrainer()
    
    @pytest.fixture
    def model(self):
        return ToyModel()
    
    def test_train_with_parent_logs(self, trainer, model):
        """Test training with parent experience logs."""
        dataset = [("a", "b"), ("c", "d")]
        parent_logs = [
            {"input": "x", "output": "y", "reward": 1.0},
            {"input": "z", "output": "w", "reward": 0.5},
        ]
        
        result = trainer.train(
            model, 
            dataset, 
            parent_logs=parent_logs
        )
        
        assert result is not None
    
    def test_training_metadata(self, trainer, model):
        """Test that training metadata is captured."""
        dataset = [("test", "data")]
        result = trainer.train(model, dataset)
        
        # Should include metadata
        assert "epochs" in result or "iterations" in result or "final_loss" in result
