#!/usr/bin/env python3
"""
IPFS Client Utility

Provides simple interface for IPFS operations:
- Add files/directories
- Get content by CID
- Pin/unpin content
- Check IPFS node status
"""

import os
import json
import hashlib
from pathlib import Path
from typing import Optional, Dict, Any, Union
import requests


class IPFSError(Exception):
    """IPFS operation error"""
    pass


class IPFSClient:
    """
    IPFS client for interacting with local or remote IPFS node.
    
    Uses the HTTP API for compatibility with go-ipfs/kubo.
    """
    
    def __init__(self, api_url: str = None):
        """
        Initialize IPFS client.
        
        Args:
            api_url: IPFS API endpoint (default: http://localhost:5001)
        """
        self.api_url = api_url or os.getenv('IPFS_API', 'http://localhost:5001')
        self.api_url = self.api_url.rstrip('/')
    
    def _request(self, endpoint: str, method: str = 'POST', **kwargs) -> Dict[str, Any]:
        """Make request to IPFS API"""
        url = f"{self.api_url}/api/v0/{endpoint}"
        try:
            if method == 'POST':
                response = requests.post(url, **kwargs, timeout=30)
            else:
                response = requests.get(url, **kwargs, timeout=30)
            
            response.raise_for_status()
            
            # Handle NDJSON responses
            if response.headers.get('Content-Type', '').startswith('application/json'):
                return response.json()
            else:
                # Try to parse as JSON anyway
                try:
                    return response.json()
                except:
                    return {'data': response.text}
                    
        except requests.RequestException as e:
            raise IPFSError(f"IPFS request failed: {e}")
    
    def is_online(self) -> bool:
        """Check if IPFS node is online"""
        try:
            self._request('id')
            return True
        except:
            return False
    
    def id(self) -> Dict[str, Any]:
        """Get IPFS node ID and info"""
        return self._request('id')
    
    def add_bytes(self, data: bytes, filename: str = 'file') -> str:
        """
        Add raw bytes to IPFS.
        
        Args:
            data: Raw bytes to add
            filename: Name for the file
        
        Returns:
            CID (Content Identifier) string
        """
        files = {'file': (filename, data)}
        result = self._request('add', files=files)
        return result.get('Hash', '')
    
    def add_str(self, content: str, filename: str = 'file.txt') -> str:
        """
        Add string content to IPFS.
        
        Args:
            content: String content to add
            filename: Name for the file
        
        Returns:
            CID string
        """
        return self.add_bytes(content.encode(), filename)
    
    def add_json(self, data: Any, filename: str = 'data.json') -> str:
        """
        Add JSON data to IPFS.
        
        Args:
            data: JSON-serializable data
            filename: Name for the file
        
        Returns:
            CID string
        """
        content = json.dumps(data, indent=2)
        return self.add_str(content, filename)
    
    def add_file(self, path: Union[str, Path]) -> str:
        """
        Add file to IPFS.
        
        Args:
            path: Path to file
        
        Returns:
            CID string
        """
        path = Path(path)
        if not path.exists():
            raise IPFSError(f"File not found: {path}")
        
        with open(path, 'rb') as f:
            return self.add_bytes(f.read(), path.name)
    
    def add_directory(self, path: Union[str, Path], recursive: bool = True) -> str:
        """
        Add directory to IPFS.
        
        Args:
            path: Path to directory
            recursive: Include subdirectories
        
        Returns:
            CID of root directory
        """
        path = Path(path)
        if not path.is_dir():
            raise IPFSError(f"Not a directory: {path}")
        
        # Collect all files
        files = []
        for file_path in path.rglob('*') if recursive else path.glob('*'):
            if file_path.is_file():
                rel_path = file_path.relative_to(path)
                with open(file_path, 'rb') as f:
                    files.append(('file', (str(rel_path), f.read())))
        
        # Add with wrap-with-directory
        result = self._request('add', files=files, params={'wrap-with-directory': 'true'})
        
        # Get root directory CID (last entry)
        if isinstance(result, dict):
            return result.get('Hash', '')
        return ''
    
    def cat(self, cid: str) -> bytes:
        """
        Get content by CID.
        
        Args:
            cid: Content Identifier
        
        Returns:
            Raw bytes content
        """
        url = f"{self.api_url}/api/v0/cat?arg={cid}"
        try:
            response = requests.post(url, timeout=30)
            response.raise_for_status()
            return response.content
        except requests.RequestException as e:
            raise IPFSError(f"Failed to get content: {e}")
    
    def cat_str(self, cid: str) -> str:
        """Get content as string"""
        return self.cat(cid).decode()
    
    def cat_json(self, cid: str) -> Any:
        """Get content as JSON"""
        return json.loads(self.cat_str(cid))
    
    def pin_add(self, cid: str) -> bool:
        """Pin content to keep it from garbage collection"""
        try:
            self._request('pin/add', params={'arg': cid})
            return True
        except:
            return False
    
    def pin_rm(self, cid: str) -> bool:
        """Unpin content"""
        try:
            self._request('pin/rm', params={'arg': cid})
            return True
        except:
            return False
    
    def pin_ls(self) -> Dict[str, Any]:
        """List pinned content"""
        return self._request('pin/ls')


class MockIPFSClient:
    """
    Mock IPFS client for testing without IPFS daemon.
    
    Stores content in memory and generates deterministic CIDs.
    """
    
    def __init__(self):
        self.storage: Dict[str, bytes] = {}
    
    def is_online(self) -> bool:
        return True
    
    def _compute_cid(self, data: bytes) -> str:
        """Compute mock CID (SHA256 hash)"""
        return 'Qm' + hashlib.sha256(data).hexdigest()[:44]
    
    def add_bytes(self, data: bytes, filename: str = 'file') -> str:
        cid = self._compute_cid(data)
        self.storage[cid] = data
        return cid
    
    def add_str(self, content: str, filename: str = 'file.txt') -> str:
        return self.add_bytes(content.encode(), filename)
    
    def add_json(self, data: Any, filename: str = 'data.json') -> str:
        return self.add_str(json.dumps(data, indent=2), filename)
    
    def add_file(self, path: Union[str, Path]) -> str:
        path = Path(path)
        with open(path, 'rb') as f:
            return self.add_bytes(f.read(), path.name)
    
    def cat(self, cid: str) -> bytes:
        if cid not in self.storage:
            raise IPFSError(f"Content not found: {cid}")
        return self.storage[cid]
    
    def cat_str(self, cid: str) -> str:
        return self.cat(cid).decode()
    
    def cat_json(self, cid: str) -> Any:
        return json.loads(self.cat_str(cid))


def get_ipfs_client(use_mock: bool = False) -> Union[IPFSClient, MockIPFSClient]:
    """
    Get appropriate IPFS client.
    
    Args:
        use_mock: Force mock client
    
    Returns:
        IPFSClient or MockIPFSClient
    """
    if use_mock:
        return MockIPFSClient()
    
    client = IPFSClient()
    if not client.is_online():
        print("Warning: IPFS node not available, using mock client")
        return MockIPFSClient()
    
    return client


# Testing
if __name__ == '__main__':
    client = get_ipfs_client()
    print(f"IPFS online: {client.is_online()}")
    
    if client.is_online():
        # Test add and cat
        test_data = "Hello, Botchain!"
        cid = client.add_str(test_data)
        print(f"Added: {cid}")
        
        retrieved = client.cat_str(cid)
        print(f"Retrieved: {retrieved}")
        
        assert test_data == retrieved
        print("Test passed!")
