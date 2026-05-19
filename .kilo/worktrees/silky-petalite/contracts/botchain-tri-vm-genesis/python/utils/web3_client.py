#!/usr/bin/env python3
"""
Web3 Client Utility

Provides interface for blockchain interactions:
- Contract deployment and interaction
- Transaction signing and submission
- Event listening
- Multi-network support
"""

import os
import json
import time
from pathlib import Path
from typing import Optional, Dict, Any, List, Union, Tuple
from dataclasses import dataclass
from eth_account import Account
from web3 import Web3
from web3.middleware import geth_poa_middleware


@dataclass
class NetworkConfig:
    """Network configuration"""
    name: str
    rpc_url: str
    chain_id: int
    explorer_url: Optional[str] = None


# Default networks
NETWORKS = {
    'localhost': NetworkConfig(
        name='Hardhat Local',
        rpc_url='http://localhost:8545',
        chain_id=31337
    ),
    'hardhat': NetworkConfig(
        name='Hardhat Docker',
        rpc_url='http://hardhat:8545',
        chain_id=31337
    ),
}


class Web3Client:
    """
    Web3 client for EVM blockchain interactions.
    
    Handles contract deployment, calls, and transactions.
    """
    
    def __init__(
        self,
        rpc_url: str = None,
        private_key: str = None,
        network: str = 'localhost'
    ):
        """
        Initialize Web3 client.
        
        Args:
            rpc_url: RPC endpoint URL (overrides network config)
            private_key: Private key for signing transactions
            network: Network name from NETWORKS
        """
        # Get RPC URL
        if rpc_url:
            self.rpc_url = rpc_url
        elif network in NETWORKS:
            self.rpc_url = NETWORKS[network].rpc_url
        else:
            self.rpc_url = os.getenv('HARDHAT_RPC_URL', 'http://localhost:8545')
        
        # Initialize Web3
        self.w3 = Web3(Web3.HTTPProvider(self.rpc_url))
        
        # Add PoA middleware for networks that need it
        self.w3.middleware_onion.inject(geth_poa_middleware, layer=0)
        
        # Set up account if private key provided
        self.account = None
        if private_key:
            self.set_account(private_key)
    
    def is_connected(self) -> bool:
        """Check if connected to network"""
        try:
            return self.w3.is_connected()
        except:
            return False
    
    def set_account(self, private_key: str):
        """Set account for signing transactions"""
        if not private_key.startswith('0x'):
            private_key = '0x' + private_key
        self.account = Account.from_key(private_key)
    
    @property
    def address(self) -> Optional[str]:
        """Get current account address"""
        return self.account.address if self.account else None
    
    def get_balance(self, address: str = None) -> int:
        """Get ETH balance in wei"""
        address = address or self.address
        if not address:
            return 0
        return self.w3.eth.get_balance(address)
    
    def get_nonce(self, address: str = None) -> int:
        """Get current nonce for address"""
        address = address or self.address
        if not address:
            return 0
        return self.w3.eth.get_transaction_count(address)
    
    def load_contract_abi(self, contract_name: str, artifacts_dir: Path = None) -> Dict:
        """Load contract ABI from Hardhat artifacts"""
        if artifacts_dir is None:
            artifacts_dir = Path(__file__).parent.parent.parent / 'hardhat' / 'artifacts' / 'contracts'
        
        # Find the artifact file
        artifact_path = artifacts_dir / f'{contract_name}.sol' / f'{contract_name}.json'
        if not artifact_path.exists():
            # Try alternative paths
            for abi_file in artifacts_dir.rglob(f'{contract_name}.json'):
                artifact_path = abi_file
                break
        
        if not artifact_path.exists():
            raise FileNotFoundError(f"Contract artifact not found: {contract_name}")
        
        with open(artifact_path) as f:
            artifact = json.load(f)
        
        return artifact
    
    def get_contract(self, address: str, abi: List[Dict]) -> Any:
        """Get contract instance"""
        return self.w3.eth.contract(address=address, abi=abi)
    
    def deploy_contract(
        self,
        contract_name: str,
        constructor_args: List = None,
        artifacts_dir: Path = None
    ) -> Tuple[str, Any]:
        """
        Deploy a contract.
        
        Args:
            contract_name: Name of the contract
            constructor_args: Constructor arguments
            artifacts_dir: Path to Hardhat artifacts
        
        Returns:
            Tuple of (contract_address, contract_instance)
        """
        if not self.account:
            raise ValueError("No account set for deployment")
        
        artifact = self.load_contract_abi(contract_name, artifacts_dir)
        abi = artifact['abi']
        bytecode = artifact['bytecode']
        
        contract = self.w3.eth.contract(abi=abi, bytecode=bytecode)
        
        # Build constructor transaction
        if constructor_args:
            tx = contract.constructor(*constructor_args).build_transaction({
                'from': self.address,
                'nonce': self.get_nonce(),
                'gas': 5000000,
                'gasPrice': self.w3.eth.gas_price
            })
        else:
            tx = contract.constructor().build_transaction({
                'from': self.address,
                'nonce': self.get_nonce(),
                'gas': 5000000,
                'gasPrice': self.w3.eth.gas_price
            })
        
        # Sign and send
        signed = self.account.sign_transaction(tx)
        tx_hash = self.w3.eth.send_raw_transaction(signed.rawTransaction)
        
        # Wait for receipt
        receipt = self.w3.eth.wait_for_transaction_receipt(tx_hash)
        
        if receipt['status'] != 1:
            raise Exception(f"Deployment failed: {receipt}")
        
        contract_address = receipt['contractAddress']
        contract_instance = self.w3.eth.contract(address=contract_address, abi=abi)
        
        return contract_address, contract_instance
    
    def call_function(
        self,
        contract: Any,
        function_name: str,
        *args
    ) -> Any:
        """Call a read-only contract function"""
        func = getattr(contract.functions, function_name)
        return func(*args).call()
    
    def send_transaction(
        self,
        contract: Any,
        function_name: str,
        *args,
        value: int = 0
    ) -> Dict:
        """
        Send a state-changing transaction.
        
        Args:
            contract: Contract instance
            function_name: Name of function to call
            args: Function arguments
            value: ETH value to send (wei)
        
        Returns:
            Transaction receipt
        """
        if not self.account:
            raise ValueError("No account set for transaction")
        
        func = getattr(contract.functions, function_name)
        
        tx = func(*args).build_transaction({
            'from': self.address,
            'nonce': self.get_nonce(),
            'gas': 500000,
            'gasPrice': self.w3.eth.gas_price,
            'value': value
        })
        
        signed = self.account.sign_transaction(tx)
        tx_hash = self.w3.eth.send_raw_transaction(signed.rawTransaction)
        
        receipt = self.w3.eth.wait_for_transaction_receipt(tx_hash)
        return dict(receipt)
    
    def approve_erc20(
        self,
        token_contract: Any,
        spender: str,
        amount: int
    ) -> Dict:
        """Approve ERC20 spending"""
        return self.send_transaction(token_contract, 'approve', spender, amount)
    
    def transfer_erc20(
        self,
        token_contract: Any,
        to: str,
        amount: int
    ) -> Dict:
        """Transfer ERC20 tokens"""
        return self.send_transaction(token_contract, 'transfer', to, amount)
    
    def get_events(
        self,
        contract: Any,
        event_name: str,
        from_block: int = 0,
        to_block: str = 'latest'
    ) -> List[Dict]:
        """Get contract events"""
        event = getattr(contract.events, event_name)
        return event.get_logs(fromBlock=from_block, toBlock=to_block)


class ContractAddresses:
    """Manages deployed contract addresses"""
    
    def __init__(self, storage_path: Path = None):
        self.storage_path = storage_path or Path(__file__).parent.parent / 'tmp' / 'addresses.json'
        self.addresses: Dict[str, str] = {}
        self._load()
    
    def _load(self):
        """Load addresses from file"""
        if self.storage_path.exists():
            with open(self.storage_path) as f:
                self.addresses = json.load(f)
    
    def _save(self):
        """Save addresses to file"""
        self.storage_path.parent.mkdir(parents=True, exist_ok=True)
        with open(self.storage_path, 'w') as f:
            json.dump(self.addresses, f, indent=2)
    
    def set(self, name: str, address: str):
        """Set contract address"""
        self.addresses[name] = address
        self._save()
    
    def get(self, name: str) -> Optional[str]:
        """Get contract address"""
        return self.addresses.get(name)
    
    def all(self) -> Dict[str, str]:
        """Get all addresses"""
        return self.addresses.copy()


# Utility functions
def to_wei(amount: float, unit: str = 'ether') -> int:
    """Convert to wei"""
    return Web3.to_wei(amount, unit)


def from_wei(amount: int, unit: str = 'ether') -> float:
    """Convert from wei"""
    return Web3.from_wei(amount, unit)


def bytes32(value: str) -> bytes:
    """Convert string to bytes32"""
    if isinstance(value, str):
        if value.startswith('0x'):
            return bytes.fromhex(value[2:])
        return value.encode().ljust(32, b'\x00')[:32]
    return value


def keccak256(data: Union[str, bytes]) -> str:
    """Compute keccak256 hash"""
    if isinstance(data, str):
        data = data.encode()
    return Web3.keccak(data).hex()


# Testing
if __name__ == '__main__':
    client = Web3Client()
    print(f"Connected: {client.is_connected()}")
    
    if client.is_connected():
        print(f"Chain ID: {client.w3.eth.chain_id}")
        print(f"Block number: {client.w3.eth.block_number}")
        
        # Test with Hardhat default account
        test_key = '0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80'
        client.set_account(test_key)
        print(f"Account: {client.address}")
        print(f"Balance: {from_wei(client.get_balance())} ETH")
