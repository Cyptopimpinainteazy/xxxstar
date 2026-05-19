#!/usr/bin/env python3
"""
Botchain Trainer

Toy deterministic trainer for AI agent fine-tuning.
Merges parent logs, runs a deterministic "training" loop, and outputs model weights.

In production, this would use PyTorch/PEFT for LoRA fine-tuning.
For this MVP, we use a deterministic text transformation to demonstrate the flow.

Usage:
    python trainer.py --parents parent_a_logs/ parent_b_logs/ --out models/child_weights.bin
"""

import argparse
import json
import hashlib
import time
import os
import struct
from pathlib import Path
from typing import List, Dict, Any, Optional
from dataclasses import dataclass, asdict
import random

# Import IPFS client
import sys
sys.path.insert(0, str(Path(__file__).parent.parent))
from utils.ipfs_client import IPFSClient


@dataclass
class TrainingConfig:
    """Training configuration"""
    epochs: int = 10
    learning_rate: float = 0.001
    batch_size: int = 32
    seed: int = 42
    model_type: str = "toy-deterministic"


@dataclass
class TrainingResult:
    """Training result metadata"""
    model_cid: str
    dataset_cid: str
    parent_cids: List[str]
    config: Dict[str, Any]
    metrics: Dict[str, float]
    timestamp: int
    trainer_version: str = "botchain-trainer-1.0.0"


class ToyModel:
    """
    Toy deterministic model for demonstration.
    
    In production, this would be replaced with:
    - PyTorch transformer with LoRA adapters
    - Hugging Face PEFT for parameter-efficient fine-tuning
    - Small language model (GPT-2 small, DistilBERT, etc.)
    """
    
    def __init__(self, seed: int = 42):
        self.seed = seed
        self.weights = {}
        self._initialize_weights()
    
    def _initialize_weights(self):
        """Initialize toy weights with deterministic values"""
        random.seed(self.seed)
        # Create deterministic "weight" vectors
        self.weights = {
            'embedding': [random.random() for _ in range(256)],
            'attention': [random.random() for _ in range(128)],
            'output': [random.random() for _ in range(64)],
            'bias': [random.random() for _ in range(32)],
        }
    
    def train(self, data: List[str], epochs: int, lr: float) -> Dict[str, float]:
        """
        Toy training loop.
        
        In production: actual gradient descent with backprop.
        Here: deterministic weight updates based on data hash.
        """
        metrics = {
            'initial_loss': 1.0,
            'final_loss': 0.0,
            'epochs_completed': 0,
        }
        
        # Compute data fingerprint for deterministic updates
        data_hash = hashlib.sha256(''.join(data).encode()).hexdigest()
        data_seed = int(data_hash[:8], 16) % (2**31)
        random.seed(data_seed)
        
        loss = 1.0
        for epoch in range(epochs):
            # Simulate loss decrease
            loss *= (1.0 - lr)
            
            # Update weights deterministically
            for key in self.weights:
                self.weights[key] = [
                    w * (1.0 - lr * 0.1) + random.random() * lr * 0.01
                    for w in self.weights[key]
                ]
            
            metrics['epochs_completed'] = epoch + 1
        
        metrics['final_loss'] = loss
        return metrics
    
    def save(self, path: Path):
        """Save model weights to binary file"""
        with open(path, 'wb') as f:
            # Write header
            f.write(b'BOTMODEL')
            f.write(struct.pack('I', 1))  # Version
            f.write(struct.pack('I', self.seed))
            
            # Write weights
            for key, values in self.weights.items():
                key_bytes = key.encode()
                f.write(struct.pack('I', len(key_bytes)))
                f.write(key_bytes)
                f.write(struct.pack('I', len(values)))
                for v in values:
                    f.write(struct.pack('f', v))
    
    @classmethod
    def load(cls, path: Path) -> 'ToyModel':
        """Load model from binary file"""
        model = cls()
        with open(path, 'rb') as f:
            # Read header
            magic = f.read(8)
            if magic != b'BOTMODEL':
                raise ValueError("Invalid model file")
            version = struct.unpack('I', f.read(4))[0]
            model.seed = struct.unpack('I', f.read(4))[0]
            
            # Read weights
            model.weights = {}
            while True:
                key_len_bytes = f.read(4)
                if not key_len_bytes:
                    break
                key_len = struct.unpack('I', key_len_bytes)[0]
                key = f.read(key_len).decode()
                num_values = struct.unpack('I', f.read(4))[0]
                values = [struct.unpack('f', f.read(4))[0] for _ in range(num_values)]
                model.weights[key] = values
        
        return model


def load_parent_logs(log_dirs: List[Path]) -> List[str]:
    """Load and merge parent training logs"""
    all_logs = []
    
    for log_dir in log_dirs:
        if not log_dir.exists():
            print(f"Warning: Log directory not found: {log_dir}")
            continue
        
        # Load all text files from log directory
        for log_file in log_dir.glob('*.txt'):
            with open(log_file, 'r', errors='ignore') as f:
                all_logs.append(f.read())
        
        for log_file in log_dir.glob('*.json'):
            with open(log_file, 'r') as f:
                data = json.load(f)
                if isinstance(data, list):
                    all_logs.extend(str(item) for item in data)
                else:
                    all_logs.append(json.dumps(data))
        
        for log_file in log_dir.glob('*.log'):
            with open(log_file, 'r', errors='ignore') as f:
                all_logs.append(f.read())
    
    return all_logs


def create_dataset(logs: List[str], output_dir: Path) -> str:
    """Create training dataset from logs and return CID"""
    dataset_path = output_dir / 'dataset.json'
    
    # Create dataset structure
    dataset = {
        'version': '1.0.0',
        'timestamp': int(time.time()),
        'num_samples': len(logs),
        'samples': logs
    }
    
    with open(dataset_path, 'w') as f:
        json.dump(dataset, f, indent=2)
    
    # Compute dataset CID (hash)
    with open(dataset_path, 'rb') as f:
        dataset_cid = hashlib.sha256(f.read()).hexdigest()
    
    return dataset_cid


def train_child_model(
    parent_log_dirs: List[Path],
    output_path: Path,
    config: TrainingConfig,
    ipfs_client: Optional[IPFSClient] = None
) -> TrainingResult:
    """
    Main training function.
    
    Args:
        parent_log_dirs: List of parent log directories
        output_path: Path for output model weights
        config: Training configuration
        ipfs_client: Optional IPFS client for uploading
    
    Returns:
        TrainingResult with CIDs and metrics
    """
    print(f"=== Botchain Trainer ===")
    print(f"Parents: {[str(p) for p in parent_log_dirs]}")
    print(f"Output: {output_path}")
    print(f"Config: {asdict(config)}")
    
    # Create output directory
    output_path.parent.mkdir(parents=True, exist_ok=True)
    tmp_dir = output_path.parent / 'tmp'
    tmp_dir.mkdir(exist_ok=True)
    
    # Load parent logs
    print("\nLoading parent logs...")
    logs = load_parent_logs(parent_log_dirs)
    print(f"  Loaded {len(logs)} log entries")
    
    # If no logs found, use placeholder data
    if not logs:
        print("  No logs found, using placeholder data")
        logs = [
            "parent_a: initialized with ethical guidelines",
            "parent_b: learned to follow commandments",
            "training: epoch 1 completed",
            "training: epoch 2 completed",
        ]
    
    # Create dataset
    print("\nCreating dataset...")
    dataset_cid = create_dataset(logs, tmp_dir)
    print(f"  Dataset CID: {dataset_cid[:16]}...")
    
    # Compute parent CIDs
    parent_cids = []
    for log_dir in parent_log_dirs:
        if log_dir.exists():
            # Hash directory contents
            dir_hash = hashlib.sha256()
            for f in sorted(log_dir.glob('*')):
                if f.is_file():
                    with open(f, 'rb') as fh:
                        dir_hash.update(fh.read())
            parent_cids.append(dir_hash.hexdigest())
    
    # Initialize and train model
    print("\nTraining model...")
    model = ToyModel(seed=config.seed)
    metrics = model.train(logs, epochs=config.epochs, lr=config.learning_rate)
    print(f"  Initial loss: {metrics['initial_loss']:.4f}")
    print(f"  Final loss: {metrics['final_loss']:.4f}")
    print(f"  Epochs: {metrics['epochs_completed']}")
    
    # Save model
    print("\nSaving model...")
    model.save(output_path)
    print(f"  Saved to: {output_path}")
    
    # Compute model CID
    with open(output_path, 'rb') as f:
        model_cid = hashlib.sha256(f.read()).hexdigest()
    print(f"  Model CID: {model_cid[:16]}...")
    
    # Upload to IPFS if client provided
    if ipfs_client:
        print("\nUploading to IPFS...")
        try:
            ipfs_model_cid = ipfs_client.add_file(output_path)
            print(f"  IPFS CID: {ipfs_model_cid}")
            model_cid = ipfs_model_cid
        except Exception as e:
            print(f"  Warning: IPFS upload failed: {e}")
    
    # Create result
    result = TrainingResult(
        model_cid=model_cid,
        dataset_cid=dataset_cid,
        parent_cids=parent_cids,
        config=asdict(config),
        metrics=metrics,
        timestamp=int(time.time())
    )
    
    # Save result metadata
    result_path = output_path.with_suffix('.json')
    with open(result_path, 'w') as f:
        json.dump(asdict(result), f, indent=2)
    print(f"\nResult metadata: {result_path}")
    
    return result


def main():
    parser = argparse.ArgumentParser(
        description='Botchain Trainer - Fine-tune child AI agents'
    )
    parser.add_argument(
        '--parents', '-p',
        nargs='+',
        required=True,
        help='Parent log directories'
    )
    parser.add_argument(
        '--out', '-o',
        required=True,
        help='Output model path'
    )
    parser.add_argument(
        '--epochs', '-e',
        type=int,
        default=10,
        help='Number of training epochs'
    )
    parser.add_argument(
        '--lr',
        type=float,
        default=0.001,
        help='Learning rate'
    )
    parser.add_argument(
        '--seed', '-s',
        type=int,
        default=42,
        help='Random seed'
    )
    parser.add_argument(
        '--ipfs',
        help='IPFS API endpoint for upload'
    )
    
    args = parser.parse_args()
    
    config = TrainingConfig(
        epochs=args.epochs,
        learning_rate=args.lr,
        seed=args.seed
    )
    
    parent_dirs = [Path(p) for p in args.parents]
    output_path = Path(args.out)
    
    ipfs_client = None
    if args.ipfs:
        ipfs_client = IPFSClient(args.ipfs)
    
    result = train_child_model(parent_dirs, output_path, config, ipfs_client)
    
    print("\n=== Training Complete ===")
    print(f"Model CID: {result.model_cid}")
    print(f"Dataset CID: {result.dataset_cid}")
    
    return result


if __name__ == '__main__':
    main()
