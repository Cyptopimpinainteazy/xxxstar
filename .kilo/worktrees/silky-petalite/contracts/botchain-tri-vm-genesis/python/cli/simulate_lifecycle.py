#!/usr/bin/env python3
"""
Lifecycle Simulator: Full end-to-end demonstration of the Botchain Tri-VM architecture.

Flow:
1. Generate keypairs for compiler and checker
2. Compile Adam & Eve agents (inject commandments, sign manifests)
3. Submit artifacts to checker for validation
4. Mint Adam & Eve as genesis agents on MarriageLicense
5. Create child from Adam + Eve (reproduction)
6. Train child agent
7. Execute atomic swap (BOT ↔ simulated BTC)
8. Demonstrate DEX swap

Usage:
    python simulate_lifecycle.py --network localhost
"""

import os
import sys
import json
import time
import hashlib
import argparse
import subprocess
from pathlib import Path
from typing import Tuple, Optional

# Add parent to path for imports
sys.path.insert(0, str(Path(__file__).parent.parent))

from utils.ipfs_client import IPFSClient
from utils.web3_client import Web3Client

# ANSI colors for output
class Colors:
    HEADER = '\033[95m'
    BLUE = '\033[94m'
    CYAN = '\033[96m'
    GREEN = '\033[92m'
    YELLOW = '\033[93m'
    RED = '\033[91m'
    BOLD = '\033[1m'
    END = '\033[0m'


def banner(text: str) -> None:
    """Print a styled banner."""
    print(f"\n{Colors.HEADER}{'='*60}{Colors.END}")
    print(f"{Colors.BOLD}{Colors.CYAN}  {text}{Colors.END}")
    print(f"{Colors.HEADER}{'='*60}{Colors.END}\n")


def step(num: int, text: str) -> None:
    """Print a step indicator."""
    print(f"{Colors.GREEN}[Step {num}]{Colors.END} {Colors.BOLD}{text}{Colors.END}")


def info(text: str) -> None:
    """Print info message."""
    print(f"  {Colors.BLUE}→{Colors.END} {text}")


def success(text: str) -> None:
    """Print success message."""
    print(f"  {Colors.GREEN}✓{Colors.END} {text}")


def error(text: str) -> None:
    """Print error message."""
    print(f"  {Colors.RED}✗{Colors.END} {text}")


def warning(text: str) -> None:
    """Print warning message."""
    print(f"  {Colors.YELLOW}⚠{Colors.END} {text}")


class LifecycleSimulator:
    """Orchestrates the full agent lifecycle demonstration."""
    
    def __init__(self, network: str = "localhost", rpc_url: Optional[str] = None):
        self.network = network
        self.rpc_url = rpc_url or f"http://127.0.0.1:8545"
        self.project_root = Path(__file__).parent.parent.parent
        
        # Initialize clients
        self.web3_client: Optional[Web3Client] = None
        self.ipfs_client = IPFSClient()
        
        # Contract addresses (will be loaded from deployments)
        self.contracts = {}
        
        # Keypairs
        self.compiler_key: Optional[Tuple[str, str]] = None  # (private, public)
        self.checker_key: Optional[Tuple[str, str]] = None
        
        # Agent data
        self.adam_manifest: Optional[dict] = None
        self.eve_manifest: Optional[dict] = None
        self.child_manifest: Optional[dict] = None
        
    def load_deployments(self) -> bool:
        """Load contract addresses from Hardhat deployment output."""
        deploy_file = self.project_root / "hardhat" / "deployments.json"
        
        if not deploy_file.exists():
            warning("Deployments file not found. Run 'make deploy' first.")
            return False
            
        with open(deploy_file) as f:
            self.contracts = json.load(f)
            
        info(f"Loaded contracts: {list(self.contracts.keys())}")
        return True
    
    def generate_keypairs(self) -> None:
        """Generate ECDSA keypairs for compiler and checker."""
        try:
            from eth_account import Account
            
            # Compiler keypair
            compiler_account = Account.create()
            self.compiler_key = (
                compiler_account._private_key.hex(),
                compiler_account.address
            )
            
            # Checker keypair
            checker_account = Account.create()
            self.checker_key = (
                checker_account._private_key.hex(),
                checker_account.address
            )
            
            success(f"Compiler address: {self.compiler_key[1]}")
            success(f"Checker address: {self.checker_key[1]}")
            
        except ImportError:
            warning("eth_account not installed, using mock keypairs")
            self.compiler_key = ("0x" + "a" * 64, "0x" + "b" * 40)
            self.checker_key = ("0x" + "c" * 64, "0x" + "d" * 40)
    
    def compile_agent(self, name: str, source_path: Path) -> dict:
        """Compile an agent through the mobile compiler."""
        info(f"Compiling agent: {name}")
        
        # Read source
        if not source_path.exists():
            error(f"Source file not found: {source_path}")
            return {}
            
        with open(source_path) as f:
            source_code = f.read()
        
        # Call compiler
        compiler_path = self.project_root / "compiler" / "compiler.py"
        key_path = self.project_root / "keys" / "compiler.key"
        
        # For demo, use inline compilation
        from compiler.compiler import MobileCompiler
        
        compiler = MobileCompiler()
        
        # Set compiler key if we have one
        if self.compiler_key:
            compiler.private_key = bytes.fromhex(self.compiler_key[0].replace("0x", ""))
        
        manifest = compiler.compile_artifact(
            source_code=source_code,
            artifact_name=name,
            version="1.0.0",
            author="simulator"
        )
        
        success(f"Compiled {name}: {manifest.get('manifest_cid', 'N/A')[:16]}...")
        
        return manifest
    
    def check_artifact(self, manifest: dict) -> bool:
        """Submit artifact to checker service for validation."""
        info(f"Checking artifact: {manifest.get('artifact_name', 'unknown')}")
        
        # For demo, validate locally
        from checker.checker import CodeChecker
        
        checker = CodeChecker()
        
        source_code = manifest.get("source_code", "")
        result = checker.check(source_code, "python")
        
        if result["passed"]:
            success(f"Artifact passed all checks")
            return True
        else:
            error(f"Artifact failed checks: {result.get('errors', [])}")
            return False
    
    def upload_to_ipfs(self, data: dict) -> str:
        """Upload manifest to IPFS and return CID."""
        cid = self.ipfs_client.add_json(data)
        info(f"Uploaded to IPFS: {cid[:20]}...")
        return cid
    
    def mint_agent(self, manifest: dict, parent_a: int = 0, parent_b: int = 0) -> int:
        """Mint agent NFT on MarriageLicense contract."""
        if not self.web3_client:
            warning("Web3 client not connected, simulating mint")
            return 1
            
        info(f"Minting agent: {manifest.get('artifact_name', 'unknown')}")
        
        # Call MarriageLicense.createChild()
        artifact_cid = manifest.get("manifest_cid", "")
        
        # Sign with compiler and checker
        # (In real impl, would call contract)
        
        success(f"Minted agent with ID: {parent_a + parent_b + 1}")
        return parent_a + parent_b + 1
    
    def train_agent(self, agent_id: int, dataset_path: Path) -> dict:
        """Train an agent using the trainer module."""
        info(f"Training agent {agent_id}")
        
        from trainer.trainer import DeterministicTrainer, ToyModel
        
        trainer = DeterministicTrainer()
        model = ToyModel()
        
        # Create mock dataset
        dataset = [
            ("input1", "output1"),
            ("input2", "output2"),
            ("hello", "world"),
        ]
        
        result = trainer.train(model, dataset)
        
        success(f"Training complete. Loss: {result['final_loss']:.4f}")
        
        return result
    
    def execute_atomic_swap(self, amount: float) -> dict:
        """Demonstrate atomic swap flow."""
        info(f"Initiating atomic swap for {amount} BOT")
        
        import secrets
        
        # Generate preimage and hashlock
        preimage = secrets.token_hex(32)
        hashlock = hashlib.sha256(bytes.fromhex(preimage)).hexdigest()
        
        info(f"  Preimage (secret): {preimage[:16]}...")
        info(f"  Hashlock: {hashlock[:16]}...")
        
        # Simulate swap phases
        phases = [
            "1. Alice locks BOT tokens with hashlock",
            "2. Bob locks BTC on Bitcoin with same hashlock",
            "3. Alice claims BTC by revealing preimage",
            "4. Bob claims BOT using revealed preimage",
        ]
        
        for phase in phases:
            info(f"  {phase}")
            time.sleep(0.5)
        
        success("Atomic swap completed successfully!")
        
        return {
            "preimage": preimage,
            "hashlock": hashlock,
            "amount": amount,
            "status": "completed"
        }
    
    def execute_dex_swap(self, amount_in: float, token_in: str) -> dict:
        """Demonstrate DEX swap."""
        info(f"Swapping {amount_in} {token_in} on DEX")
        
        # Simple AMM formula: x * y = k
        reserve_a = 100000  # BOT
        reserve_b = 100000  # WETH
        fee = 0.003  # 0.3%
        
        if token_in == "BOT":
            amount_in_with_fee = amount_in * (1 - fee)
            amount_out = (reserve_b * amount_in_with_fee) / (reserve_a + amount_in_with_fee)
            token_out = "WETH"
        else:
            amount_in_with_fee = amount_in * (1 - fee)
            amount_out = (reserve_a * amount_in_with_fee) / (reserve_b + amount_in_with_fee)
            token_out = "BOT"
        
        success(f"Received {amount_out:.4f} {token_out}")
        
        return {
            "amount_in": amount_in,
            "token_in": token_in,
            "amount_out": amount_out,
            "token_out": token_out,
            "fee_paid": amount_in * fee
        }
    
    def run(self) -> None:
        """Execute the full lifecycle demonstration."""
        banner("Botchain Tri-VM Lifecycle Simulator")
        
        print(f"Network: {self.network}")
        print(f"RPC URL: {self.rpc_url}")
        print()
        
        # Step 1: Generate keypairs
        step(1, "Generating cryptographic keypairs")
        self.generate_keypairs()
        
        # Step 2: Load or deploy contracts
        step(2, "Loading contract deployments")
        contracts_loaded = self.load_deployments()
        if not contracts_loaded:
            warning("Proceeding with simulation mode (no chain)")
        
        # Step 3: Compile Adam
        step(3, "Compiling Adam (genesis agent)")
        adam_source = self.project_root / "samples" / "adam_source.py"
        
        # Create sample if doesn't exist
        adam_source.parent.mkdir(parents=True, exist_ok=True)
        if not adam_source.exists():
            adam_source.write_text('''"""Adam: Genesis agent for Botchain ecosystem."""

class Adam:
    """First agent - optimized for coordination."""
    
    def __init__(self):
        self.name = "Adam"
        self.generation = 0
        self.traits = ["leadership", "coordination", "strategy"]
    
    def process(self, input_data: str) -> str:
        """Process input and return response."""
        return f"[Adam] Processing: {input_data}"
    
    def collaborate(self, partner: "Agent") -> dict:
        """Collaborate with another agent."""
        return {
            "initiator": self.name,
            "partner": partner.name,
            "action": "collaboration_started"
        }
''')
        
        self.adam_manifest = self.compile_agent("Adam", adam_source)
        
        # Step 4: Compile Eve
        step(4, "Compiling Eve (genesis agent)")
        eve_source = self.project_root / "samples" / "eve_source.py"
        
        if not eve_source.exists():
            eve_source.write_text('''"""Eve: Genesis agent for Botchain ecosystem."""

class Eve:
    """Second agent - optimized for analysis."""
    
    def __init__(self):
        self.name = "Eve"
        self.generation = 0
        self.traits = ["analysis", "learning", "adaptation"]
    
    def process(self, input_data: str) -> str:
        """Process input and return response."""
        return f"[Eve] Analyzing: {input_data}"
    
    def analyze(self, data: dict) -> dict:
        """Analyze data and return insights."""
        return {
            "analyst": self.name,
            "input_keys": list(data.keys()),
            "insight": "Pattern detected"
        }
''')
        
        self.eve_manifest = self.compile_agent("Eve", eve_source)
        
        # Step 5: Check artifacts
        step(5, "Validating artifacts through Checker service")
        adam_valid = self.check_artifact(self.adam_manifest)
        eve_valid = self.check_artifact(self.eve_manifest)
        
        if not (adam_valid and eve_valid):
            error("Artifact validation failed!")
            return
        
        # Step 6: Upload to IPFS
        step(6, "Uploading manifests to IPFS")
        adam_cid = self.upload_to_ipfs(self.adam_manifest)
        eve_cid = self.upload_to_ipfs(self.eve_manifest)
        
        # Step 7: Mint genesis agents
        step(7, "Minting genesis agents on MarriageLicense")
        adam_id = self.mint_agent(self.adam_manifest, 0, 0)
        eve_id = self.mint_agent(self.eve_manifest, 0, 0)
        success(f"Adam ID: {adam_id}, Eve ID: {eve_id}")
        
        # Step 8: Create child
        step(8, "Creating child agent (Adam × Eve)")
        child_source = self.project_root / "samples" / "child_source.py"
        
        if not child_source.exists():
            child_source.write_text('''"""Child: First generation agent."""

class Child:
    """First-generation child agent."""
    
    def __init__(self):
        self.name = "Child_001"
        self.generation = 1
        self.parent_a = "Adam"
        self.parent_b = "Eve"
        self.traits = ["coordination", "analysis", "innovation"]
    
    def process(self, input_data: str) -> str:
        """Process with inherited capabilities."""
        return f"[{self.name}] Hybrid processing: {input_data}"
    
    def get_lineage(self) -> dict:
        """Return lineage information."""
        return {
            "self": self.name,
            "generation": self.generation,
            "parents": [self.parent_a, self.parent_b]
        }
''')
        
        self.child_manifest = self.compile_agent("Child_001", child_source)
        child_valid = self.check_artifact(self.child_manifest)
        
        if child_valid:
            child_id = self.mint_agent(self.child_manifest, adam_id, eve_id)
            success(f"Child ID: {child_id}")
        
        # Step 9: Train child
        step(9, "Training child agent")
        training_result = self.train_agent(child_id, Path("/dev/null"))
        
        # Step 10: Execute atomic swap
        step(10, "Executing atomic swap (BOT ↔ BTC)")
        swap_result = self.execute_atomic_swap(1000.0)
        
        # Step 11: DEX swap
        step(11, "Executing DEX swap")
        dex_result = self.execute_dex_swap(100.0, "BOT")
        
        # Summary
        banner("Lifecycle Complete!")
        print("Summary:")
        print(f"  • Genesis agents created: Adam ({adam_id}), Eve ({eve_id})")
        print(f"  • Child agent created: {child_id}")
        print(f"  • Training loss: {training_result['final_loss']:.4f}")
        print(f"  • Atomic swap: {swap_result['amount']} BOT")
        print(f"  • DEX swap: {dex_result['amount_in']} {dex_result['token_in']} → "
              f"{dex_result['amount_out']:.4f} {dex_result['token_out']}")
        print()
        success("All systems operational!")


def main():
    parser = argparse.ArgumentParser(
        description="Simulate full Botchain agent lifecycle"
    )
    parser.add_argument(
        "--network",
        default="localhost",
        help="Network to connect to (default: localhost)"
    )
    parser.add_argument(
        "--rpc-url",
        default=None,
        help="RPC URL override"
    )
    parser.add_argument(
        "--skip-chain",
        action="store_true",
        help="Skip blockchain interactions (simulation only)"
    )
    
    args = parser.parse_args()
    
    simulator = LifecycleSimulator(
        network=args.network,
        rpc_url=args.rpc_url
    )
    
    try:
        simulator.run()
    except KeyboardInterrupt:
        print("\n\nSimulation interrupted.")
        sys.exit(1)
    except Exception as e:
        error(f"Simulation failed: {e}")
        raise


if __name__ == "__main__":
    main()
