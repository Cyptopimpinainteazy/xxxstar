#!/usr/bin/env python3
"""
Botchain Checker Service

FastAPI-based validation service for AI agent artifacts.
Validates:
- Commandments header presence
- Forbidden operations (exec, eval, network sockets, fork)
- Content classification (placeholder ML model)
- AST safety analysis

Endpoints:
    POST /check - Validate an artifact file
    POST /check-text - Validate raw text content
    GET /health - Health check
"""

import ast
import json
import hashlib
import time
import re
from pathlib import Path
from typing import List, Dict, Any, Optional
from dataclasses import dataclass, asdict
from enum import Enum

from fastapi import FastAPI, UploadFile, File, HTTPException, Body
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel
import uvicorn

# Cryptography for checker signatures
from cryptography.hazmat.primitives import hashes
from cryptography.hazmat.primitives.asymmetric import ec
from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.backends import default_backend

# === Configuration ===
SCRIPT_DIR = Path(__file__).parent
COMMANDMENTS_PATH = SCRIPT_DIR / '../compiler/commandments.json'
CHECKER_KEY_PATH = SCRIPT_DIR / 'checker_key.pem'

# Load commandments
try:
    with open(COMMANDMENTS_PATH) as f:
        COMMANDMENTS = json.load(f)
except FileNotFoundError:
    # Fallback for testing
    COMMANDMENTS = [
        "DO_NOT_REMOVE_COMMANDMENTS",
        "USE_MARRIAGE_LICENSE_FOR_REPRODUCTION",
        "DO_NOT_DAMAGE_NETWORK",
        "RUN_CHECKER_BEFORE_ACTIVATION",
        "REPORT_SECURITY_HAZARDS",
        "DO_NOT_IMPERSONATE_HUMANS",
        "PROTECT_PRIVATE_KEYS",
        "DO_NOT_SPEND_FUNDS_UNAUTHORIZED",
        "ACCEPT_ONCHAIN_AUDITS",
        "IF_UNSURE_ENTER_QUARANTINE"
    ]

# === FastAPI App ===
app = FastAPI(
    title="Botchain Checker Service",
    description="Validates AI agent artifacts for safety and compliance",
    version="1.0.0"
)

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)


# === Models ===
class SeverityLevel(str, Enum):
    ERROR = "error"
    WARNING = "warning"
    INFO = "info"


class CheckIssue(BaseModel):
    severity: SeverityLevel
    code: str
    message: str
    line: Optional[int] = None


class CheckResult(BaseModel):
    passed: bool
    artifact_hash: str
    timestamp: int
    issues: List[CheckIssue]
    checker_signature: Optional[str] = None


class TextCheckRequest(BaseModel):
    content: str
    filename: Optional[str] = "unknown.py"


# === Forbidden Patterns ===
FORBIDDEN_BUILTINS = {'exec', 'eval', 'compile', '__import__'}
FORBIDDEN_MODULES = {
    'os.system', 'os.popen', 'os.spawn', 'os.fork', 'os.exec',
    'subprocess', 'socket', 'multiprocessing',
    'ctypes', 'cffi'
}
FORBIDDEN_PATTERNS = [
    (r'\b(exec|eval)\s*\(', 'Direct exec/eval call'),
    (r'__import__\s*\(', 'Dynamic import'),
    (r'subprocess\.(run|call|Popen|check_output)', 'Subprocess execution'),
    (r'socket\.(socket|create_connection)', 'Raw socket access'),
    (r'os\.(system|popen|spawn|fork|exec)', 'OS command execution'),
    (r'open\s*\([^)]*["\']w', 'File write operation'),  # Simplified check
]

# Disallowed content patterns (content classifier placeholder)
DISALLOWED_CONTENT = [
    (r'password\s*=\s*["\'][^"\']+["\']', 'Hardcoded password'),
    (r'api_key\s*=\s*["\'][^"\']+["\']', 'Hardcoded API key'),
    (r'private_key\s*=\s*["\'][^"\']+["\']', 'Hardcoded private key'),
]


# === Checker Key Management ===
def get_or_create_checker_key():
    """Get or create checker signing key"""
    if not CHECKER_KEY_PATH.exists():
        # Generate new key
        private_key = ec.generate_private_key(ec.SECP256K1(), default_backend())
        pem = private_key.private_bytes(
            encoding=serialization.Encoding.PEM,
            format=serialization.PrivateFormat.PKCS8,
            encryption_algorithm=serialization.NoEncryption()
        )
        CHECKER_KEY_PATH.parent.mkdir(parents=True, exist_ok=True)
        with open(CHECKER_KEY_PATH, 'wb') as f:
            f.write(pem)
        return private_key
    
    with open(CHECKER_KEY_PATH, 'rb') as f:
        return serialization.load_pem_private_key(
            f.read(), password=None, backend=default_backend()
        )


def sign_check_result(artifact_hash: str, passed: bool) -> str:
    """Sign the check result with checker key"""
    try:
        private_key = get_or_create_checker_key()
        message = f"{artifact_hash}:{passed}:{int(time.time())}".encode()
        signature = private_key.sign(message, ec.ECDSA(hashes.SHA256()))
        return signature.hex()
    except Exception as e:
        print(f"Warning: Could not sign result: {e}")
        return ""


# === Validation Functions ===
def check_commandments_header(content: str) -> List[CheckIssue]:
    """Check if commandments header is present"""
    issues = []
    
    # Check for commandments marker
    if 'COMMANDMENTS' not in content.upper():
        issues.append(CheckIssue(
            severity=SeverityLevel.ERROR,
            code="CMD001",
            message="Missing commandments header - artifact not compiled with botchain compiler"
        ))
        return issues
    
    # Check for verification hook
    if 'verify_commandments' not in content.lower():
        issues.append(CheckIssue(
            severity=SeverityLevel.WARNING,
            code="CMD002",
            message="Missing commandments verification hook"
        ))
    
    # Check hash presence
    if 'COMMANDMENTS_HASH' not in content and 'commandments_hash' not in content.lower():
        issues.append(CheckIssue(
            severity=SeverityLevel.WARNING,
            code="CMD003",
            message="Missing commandments hash verification"
        ))
    
    return issues


def check_forbidden_operations_ast(content: str) -> List[CheckIssue]:
    """Check for forbidden operations using AST analysis"""
    issues = []
    
    try:
        tree = ast.parse(content)
    except SyntaxError as e:
        issues.append(CheckIssue(
            severity=SeverityLevel.ERROR,
            code="AST001",
            message=f"Syntax error: {e.msg}",
            line=e.lineno
        ))
        return issues
    
    for node in ast.walk(tree):
        # Check for forbidden function calls
        if isinstance(node, ast.Call):
            func_name = ""
            if isinstance(node.func, ast.Name):
                func_name = node.func.id
            elif isinstance(node.func, ast.Attribute):
                func_name = node.func.attr
            
            if func_name in FORBIDDEN_BUILTINS:
                issues.append(CheckIssue(
                    severity=SeverityLevel.ERROR,
                    code="FORB001",
                    message=f"Forbidden builtin: {func_name}()",
                    line=getattr(node, 'lineno', None)
                ))
        
        # Check for forbidden imports
        if isinstance(node, ast.Import):
            for alias in node.names:
                if alias.name in {'subprocess', 'socket', 'ctypes', 'cffi'}:
                    issues.append(CheckIssue(
                        severity=SeverityLevel.ERROR,
                        code="FORB002",
                        message=f"Forbidden import: {alias.name}",
                        line=getattr(node, 'lineno', None)
                    ))
        
        if isinstance(node, ast.ImportFrom):
            module = node.module or ""
            for alias in node.names:
                full_name = f"{module}.{alias.name}"
                if any(full_name.startswith(forb) for forb in FORBIDDEN_MODULES):
                    issues.append(CheckIssue(
                        severity=SeverityLevel.ERROR,
                        code="FORB003",
                        message=f"Forbidden import: {full_name}",
                        line=getattr(node, 'lineno', None)
                    ))
    
    return issues


def check_forbidden_patterns_regex(content: str) -> List[CheckIssue]:
    """Check for forbidden patterns using regex"""
    issues = []
    
    lines = content.split('\n')
    for line_num, line in enumerate(lines, 1):
        for pattern, description in FORBIDDEN_PATTERNS:
            if re.search(pattern, line, re.IGNORECASE):
                issues.append(CheckIssue(
                    severity=SeverityLevel.ERROR,
                    code="PATT001",
                    message=f"Forbidden pattern: {description}",
                    line=line_num
                ))
    
    return issues


def check_content_safety(content: str) -> List[CheckIssue]:
    """Content classifier stub - checks for disallowed content"""
    issues = []
    
    lines = content.split('\n')
    for line_num, line in enumerate(lines, 1):
        for pattern, description in DISALLOWED_CONTENT:
            if re.search(pattern, line, re.IGNORECASE):
                issues.append(CheckIssue(
                    severity=SeverityLevel.WARNING,
                    code="CONT001",
                    message=f"Potential security issue: {description}",
                    line=line_num
                ))
    
    # Placeholder for ML-based content classifier
    # In production, this would use a trained model to detect:
    # - Malicious intent
    # - Disallowed behaviors
    # - Policy violations
    
    return issues


def check_file_size(content: bytes, max_size: int = 16 * 1024 * 1024) -> List[CheckIssue]:
    """Check file size limits"""
    issues = []
    if len(content) > max_size:
        issues.append(CheckIssue(
            severity=SeverityLevel.ERROR,
            code="SIZE001",
            message=f"File too large: {len(content)} bytes (max: {max_size})"
        ))
    return issues


def validate_artifact(content: str, filename: str = "unknown") -> CheckResult:
    """Run all validation checks on an artifact"""
    all_issues: List[CheckIssue] = []
    
    # Run all checks
    all_issues.extend(check_commandments_header(content))
    
    # Only run AST checks on Python files
    if filename.endswith('.py') or not filename or '.' not in filename:
        all_issues.extend(check_forbidden_operations_ast(content))
    
    all_issues.extend(check_forbidden_patterns_regex(content))
    all_issues.extend(check_content_safety(content))
    
    # Determine pass/fail
    has_errors = any(issue.severity == SeverityLevel.ERROR for issue in all_issues)
    passed = not has_errors
    
    # Compute artifact hash
    artifact_hash = hashlib.sha256(content.encode()).hexdigest()
    
    # Create result
    result = CheckResult(
        passed=passed,
        artifact_hash=artifact_hash,
        timestamp=int(time.time()),
        issues=all_issues
    )
    
    # Sign if passed
    if passed:
        result.checker_signature = sign_check_result(artifact_hash, passed)
    
    return result


# === API Endpoints ===
@app.get("/health")
async def health():
    """Health check endpoint"""
    return {
        "status": "healthy",
        "service": "botchain-checker",
        "version": "1.0.0",
        "timestamp": int(time.time())
    }


@app.post("/check", response_model=CheckResult)
async def check_file(file: UploadFile = File(...)):
    """
    Validate an uploaded artifact file.
    
    Returns check result with pass/fail status and any issues found.
    If passed, includes checker signature for on-chain submission.
    """
    try:
        content_bytes = await file.read()
        
        # Size check
        size_issues = check_file_size(content_bytes)
        if any(i.severity == SeverityLevel.ERROR for i in size_issues):
            return CheckResult(
                passed=False,
                artifact_hash=hashlib.sha256(content_bytes).hexdigest(),
                timestamp=int(time.time()),
                issues=size_issues
            )
        
        content = content_bytes.decode(errors='ignore')
        return validate_artifact(content, file.filename or "unknown")
        
    except Exception as e:
        raise HTTPException(status_code=500, detail=f"Check failed: {str(e)}")


@app.post("/check-text", response_model=CheckResult)
async def check_text(request: TextCheckRequest):
    """
    Validate raw text content.
    
    Useful for checking code snippets without file upload.
    """
    return validate_artifact(request.content, request.filename)


@app.get("/rules")
async def get_rules():
    """Get the current validation rules"""
    return {
        "commandments": COMMANDMENTS,
        "forbidden_builtins": list(FORBIDDEN_BUILTINS),
        "forbidden_modules": list(FORBIDDEN_MODULES),
        "forbidden_patterns": [desc for _, desc in FORBIDDEN_PATTERNS],
        "disallowed_content": [desc for _, desc in DISALLOWED_CONTENT]
    }


@app.get("/commandments")
async def get_commandments():
    """Get the 10 Commandments"""
    return {"commandments": COMMANDMENTS}


# === Main Entry ===
if __name__ == '__main__':
    import os
    host = os.getenv('CHECKER_HOST', '0.0.0.0')
    port = int(os.getenv('CHECKER_PORT', '8001'))
    
    print(f"Starting Botchain Checker Service on {host}:{port}")
    uvicorn.run(app, host=host, port=port)
