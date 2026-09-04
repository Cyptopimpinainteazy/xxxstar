import os
import json
import shutil
import random
import string
import subprocess
from contextlib import asynccontextmanager
from datetime import datetime, timezone
from typing import Optional

import httpx
from fastapi import FastAPI, Depends, HTTPException, Query, WebSocket, WebSocketDisconnect
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel
from sqlalchemy import select, func, desc
from sqlalchemy.ext.asyncio import AsyncSession

from database import init_db, get_session, Block, Transaction, Account, Contract, Project

REPO_BASE = os.environ.get("X3_REPO_BASE", os.path.abspath(os.path.join(BASE_DIR := os.path.dirname(os.path.abspath(__file__)), "..", "..", "..")))
X3_RPC = os.environ.get("X3_RPC_URL", "http://127.0.0.1:9933")
SWARM_API = os.environ.get("SWARM_API_URL", "http://127.0.0.1:8787")


@asynccontextmanager
async def lifespan(app: FastAPI):
    await init_db()
    yield


app = FastAPI(title="Super IDE API", version="0.1.0", lifespan=lifespan)

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_methods=["*"],
    allow_headers=["*"],
)


def _r() -> str:
    return "0x" + "".join(random.choices(string.hexdigits, k=64)).lower()


def _a() -> str:
    return "0x" + "".join(random.choices(string.hexdigits, k=40)).lower()


SAFE_DIRS = [
    os.path.join(REPO_BASE, "X3-contracts"),
    os.path.join(REPO_BASE, "x3-templates"),
    os.path.join(REPO_BASE, "x3-lang"),
    os.path.join(REPO_BASE, "apps", "super-ide"),
    os.path.join(REPO_BASE, "packages"),
    os.path.join(REPO_BASE, "crates"),
]


def _resolve_path(relative: str) -> str:
    path = os.path.normpath(os.path.join(REPO_BASE, relative.lstrip("/")))
    if not any(path.startswith(sd) for sd in SAFE_DIRS):
        raise HTTPException(403, f"Access denied: path outside safe directories")
    if not path.startswith(REPO_BASE):
        raise HTTPException(403, "Access denied: path outside repo")
    return path


# ─── Health ─────────────────────────────────────────


@app.get("/api/health")
async def health():
    return {"status": "ok", "service": "super-ide-api", "repo": REPO_BASE, "timestamp": datetime.now(timezone.utc).isoformat()}


# ─── Network Status ────────────────────────────────


@app.get("/api/network/status")
async def network_status():
    try:
        async with httpx.AsyncClient(timeout=2) as client:
            r = await client.post(X3_RPC, json={"jsonrpc": "2.0", "id": 1, "method": "system_health", "params": []})
            health_data = r.json().get("result", {})
        r2 = await client.post(X3_RPC, json={"jsonrpc": "2.0", "id": 2, "method": "system_properties", "params": []})
        props = r2.json().get("result", {})
        r3 = await client.post(X3_RPC, json={"jsonrpc": "2.0", "id": 3, "method": "chain_getFinalizedHead", "params": []})
        head = r3.json().get("result", "0x")
    except Exception:
        health_data = {"peers": 0, "isSyncing": True, "shouldHavePeers": False}
        props = {}
        head = "0x"

    return {
        "peers": health_data.get("peers", 0),
        "syncing": health_data.get("isSyncing", True),
        "bestBlock": 0,
        "chain": props.get("chainType", "X3 Chain"),
        "tokenSymbol": props.get("tokenSymbol", "X3"),
        "ss58Format": props.get("ss58Format", 42),
        "finalizedHead": head,
        "rpcUrl": X3_RPC,
    }


# ─── RPC Proxy ─────────────────────────────────────


class RpcRequest(BaseModel):
    jsonrpc: str = "2.0"
    method: str
    params: list = []
    id: int = 1


@app.post("/api/rpc")
async def rpc_proxy(req: RpcRequest):
    try:
        async with httpx.AsyncClient(timeout=10) as client:
            r = await client.post(X3_RPC, json=req.model_dump())
            return r.json()
    except Exception as e:
        raise HTTPException(502, f"RPC unavailable: {str(e)}")


# ─── File System ────────────────────────────────────


@app.get("/api/files")
async def list_dir(path: str = Query(".")):
    full = _resolve_path(path)
    if not os.path.isdir(full):
        raise HTTPException(400, "Not a directory")
    entries = []
    for name in sorted(os.listdir(full)):
        if name.startswith("."):
            continue
        fpath = os.path.join(full, name)
        rel = os.path.relpath(fpath, REPO_BASE)
        entries.append({
            "name": name,
            "path": rel,
            "type": "dir" if os.path.isdir(fpath) else "file",
            "size": os.path.getsize(fpath) if os.path.isfile(fpath) else 0,
        })
    return entries


@app.get("/api/files/read")
async def read_file(path: str = Query(...)):
    full = _resolve_path(path)
    if not os.path.isfile(full):
        raise HTTPException(404, "File not found")
    try:
        with open(full) as f:
            content = f.read()
    except UnicodeDecodeError:
        raise HTTPException(400, "Binary file cannot be read as text")
    return {"path": path, "content": content, "size": os.path.getsize(full)}


class FileWriteRequest(BaseModel):
    path: str
    content: str


@app.post("/api/files/write")
async def write_file(req: FileWriteRequest):
    full = _resolve_path(req.path)
    os.makedirs(os.path.dirname(full), exist_ok=True)
    with open(full, "w") as f:
        f.write(req.content)
    return {"path": req.path, "written": os.path.getsize(full)}


# ─── Templates ──────────────────────────────────────


@app.get("/api/templates")
async def list_templates():
    tmpl_dir = _resolve_path("x3-templates")
    if not os.path.isdir(tmpl_dir):
        return []
    templates = []
    for name in sorted(os.listdir(tmpl_dir)):
        if name.endswith(".x3"):
            fpath = os.path.join(tmpl_dir, name)
            with open(fpath) as f:
                content = f.read()
            lines = content.split("\n")
            desc = ""
            for line in lines[:20]:
                if line.startswith("///") or line.startswith("//"):
                    desc += line.lstrip("/ ").strip() + " "
            templates.append({
                "name": name.replace(".x3", ""),
                "filename": name,
                "path": os.path.join("x3-templates", name),
                "description": desc.strip() or f"{name.replace('.x3', '').replace('_', ' ').title()} template",
                "size": os.path.getsize(fpath),
                "lines": len(lines),
            })
    return templates


@app.get("/api/templates/{name}")
async def get_template(name: str):
    tmpl_path = _resolve_path(f"x3-templates/{name}.x3")
    if not os.path.isfile(tmpl_path):
        raise HTTPException(404, "Template not found")
    with open(tmpl_path) as f:
        content = f.read()
    return {"name": name, "content": content, "path": f"x3-templates/{name}.x3"}


class ScaffoldRequest(BaseModel):
    template: str
    project_name: str


@app.post("/api/templates/scaffold")
async def scaffold_project(req: ScaffoldRequest, db: AsyncSession = Depends(get_session)):
    tmpl_path = _resolve_path(f"x3-templates/{req.template}.x3")
    if not os.path.isfile(tmpl_path):
        raise HTTPException(404, "Template not found")
    proj_dir = os.path.join(REPO_BASE, "apps", "super-ide", "projects", req.project_name)
    if os.path.exists(proj_dir):
        raise HTTPException(409, "Project already exists")
    os.makedirs(proj_dir, exist_ok=True)
    shutil.copy2(tmpl_path, os.path.join(proj_dir, f"{req.project_name}.x3"))
    with open(os.path.join(proj_dir, "README.md"), "w") as f:
        f.write(f"# {req.project_name}\n\nScaffolded from **{req.template}** template.\n")
    db.add(Project(name=req.project_name, path=proj_dir, template=req.template))
    await db.commit()
    return {
        "name": req.project_name,
        "path": os.path.relpath(proj_dir, REPO_BASE),
        "template": req.template,
        "files": [f"{req.project_name}.x3", "README.md"],
    }


# ─── ABI Browser ────────────────────────────────────


@app.get("/api/abis")
async def list_abis():
    out_dir = _resolve_path("X3-contracts/evm/out")
    if not os.path.isdir(out_dir):
        return []
    abis = []
    for contract_dir in sorted(os.listdir(out_dir)):
        contract_path = os.path.join(out_dir, contract_dir)
        if not os.path.isdir(contract_path):
            continue
        base_name = contract_dir.replace(".sol", "").replace(".vy", "")
        json_file = os.path.join(contract_path, f"{base_name}.json")
        if not os.path.isfile(json_file):
            continue
        try:
            with open(json_file) as f:
                data = json.load(f)
            abi = data.get("abi", [])
            bytecode = data.get("bytecode", {}).get("object", "")
            methods = [
                {"name": item.get("name", ""), "type": item.get("type", ""),
                 "stateMutability": item.get("stateMutability", "")}
                for item in abi if item.get("type") in ("function", "event", "constructor")
            ]
            abis.append({
                "name": base_name,
                "path": os.path.relpath(json_file, REPO_BASE),
                "methods": methods,
                "hasBytecode": bool(bytecode),
                "abiCount": len(abi),
            })
        except Exception:
            pass
    return abis


@app.get("/api/abis/{name}")
async def get_abi(name: str):
    candidates = [
        f"X3-contracts/evm/out/{name}.sol/{name}.json",
        f"X3-contracts/evm/out/{name}/{name}.json",
    ]
    json_file = None
    for c in candidates:
        try:
            fp = _resolve_path(c)
            if os.path.isfile(fp):
                json_file = fp
                break
        except HTTPException:
            continue
    if not json_file:
        raise HTTPException(404, "ABI not found")
    with open(json_file) as f:
        data = json.load(f)
    return {
        "name": name,
        "abi": data.get("abi", []),
        "bytecode": data.get("bytecode", {}),
        "deployedBytecode": data.get("deployedBytecode", {}),
        "metadata": data.get("metadata", ""),
    }


# ─── Projects ───────────────────────────────────────


@app.get("/api/projects")
async def list_projects(db: AsyncSession = Depends(get_session)):
    result = await db.execute(select(Project).order_by(desc(Project.id)))
    return [
        {"id": p.id, "name": p.name, "path": p.path, "template": p.template,
         "createdAt": p.created_at.isoformat()}
        for p in result.scalars().all()
    ]


# ─── X3 Compiler ────────────────────────────────────


class CompileRequest(BaseModel):
    code: str
    language: str = "x3"  # "x3", "solidity", "rust"


@app.post("/api/compile")
async def compile_code(req: CompileRequest):
    result = {"success": False, "output": "", "errors": "", "warnings": ""}
    if req.language == "x3":
        x3_cli = os.path.join(REPO_BASE, "x3-lang", "cli.py")
        if os.path.exists(x3_cli):
            try:
                proc = subprocess.run(
                    ["python3", x3_cli, "compile", "--stdin"],
                    input=req.code, capture_output=True, text=True, timeout=10,
                    cwd=os.path.join(REPO_BASE, "x3-lang"),
                )
                result["success"] = proc.returncode == 0
                result["output"] = proc.stdout
                result["errors"] = proc.stderr
            except subprocess.TimeoutExpired:
                result["errors"] = "Compilation timed out"
            except Exception as e:
                result["errors"] = str(e)
        else:
            result["output"] = "X3 compiler not available. Syntax check only.\n\n"
            result["warnings"] = "Compiler CLI not found at x3-lang/cli.py"
            result["success"] = True
    elif req.language == "solidity":
        result["output"] = "Solidity compilation requires Foundry. Use `forge build` externally."
        result["success"] = True
    elif req.language == "rust":
        result["output"] = "Rust compilation requires cargo. Use `cargo build` externally."
        result["success"] = True
    return result


# ─── Key Management ────────────────────────────────


class KeyGenerateRequest(BaseModel):
    key_type: str = "ed25519"
    label: str = ""


@app.post("/api/keys/generate")
async def generate_key(req: KeyGenerateRequest, db: AsyncSession = Depends(get_session)):
    try:
        import nacl.bindings
        seed = os.urandom(32)
        pk = nacl.bindings.crypto_sign_seed_keypair(seed)[0]
        address = "0x" + pk.hex()[:40]
        public_key = "0x" + pk.hex()
        label = req.label or f"Account-{address[:8]}"
        db.add(Account(address=address, public_key=public_key, key_type=req.key_type, label=label))
        await db.commit()
        return {
            "address": address,
            "publicKey": public_key,
            "label": label,
            "keyType": req.key_type,
            "seed": "0x" + seed.hex(),
        }
    except ImportError:
        seed = os.urandom(32)
        address = "0x" + seed.hex()[:40]
        pk = "0x" + seed.hex()[:64]
        label = req.label or f"Account-{address[:8]}"
        db.add(Account(address=address, public_key=pk, key_type=req.key_type, label=label))
        await db.commit()
        return {
            "address": address,
            "publicKey": pk,
            "label": label,
            "keyType": req.key_type,
            "seed": "0x" + seed.hex(),
        }


# ─── Explorer (Blocks / Txns) ────────────────────


async def _seed_blocks(db: AsyncSession, count: int = 10):
    result = await db.execute(select(func.count()).select_from(Block))
    if result.scalar() > 0:
        return
    now = datetime.now(timezone.utc)
    for i in range(count, 0, -1):
        db.add(Block(number=1_000_000 + i, hash=_r(), timestamp=now, tx_count=random.randint(0, 15), producer=_a()))
    await db.commit()


@app.get("/api/explorer/blocks")
async def get_blocks(limit: int = 20, offset: int = 0, db: AsyncSession = Depends(get_session)):
    await _seed_blocks(db)
    result = await db.execute(select(Block).order_by(desc(Block.number)).offset(offset).limit(limit))
    return [{"number": b.number, "hash": b.hash, "timestamp": b.timestamp.isoformat(), "txCount": b.tx_count, "producer": b.producer} for b in result.scalars().all()]


@app.get("/api/explorer/blocks/{number}")
async def get_block(number: int, db: AsyncSession = Depends(get_session)):
    result = await db.execute(select(Block).where(Block.number == number))
    block = result.scalar_one_or_none()
    if not block:
        raise HTTPException(404, "Block not found")
    return {"number": block.number, "hash": block.hash, "timestamp": block.timestamp.isoformat(), "txCount": block.tx_count, "producer": block.producer}


async def _seed_txs(db: AsyncSession, count: int = 20):
    result = await db.execute(select(func.count()).select_from(Transaction))
    if result.scalar() > 0:
        return
    now = datetime.now(timezone.utc)
    for i in range(count):
        db.add(Transaction(hash=_r(), block_number=1_000_000 + random.randint(1, 10),
                from_address=_a(), to_address=_a(), value=str(random.randint(1, 10000)),
                data="0x", gas_limit=21000, gas_price=str(random.randint(1, 100)),
                status=random.choice(["confirmed", "confirmed", "confirmed", "pending"]), timestamp=now))
    await db.commit()


@app.get("/api/explorer/transactions")
async def get_transactions(limit: int = 20, offset: int = 0, db: AsyncSession = Depends(get_session)):
    await _seed_txs(db)
    result = await db.execute(select(Transaction).order_by(desc(Transaction.id)).offset(offset).limit(limit))
    return [{"hash": tx.hash, "blockNumber": tx.block_number, "from": tx.from_address, "to": tx.to_address,
             "value": tx.value, "status": tx.status, "timestamp": tx.timestamp.isoformat()} for tx in result.scalars().all()]


@app.get("/api/explorer/transactions/{hash}")
async def get_transaction(hash: str, db: AsyncSession = Depends(get_session)):
    result = await db.execute(select(Transaction).where(Transaction.hash == hash))
    tx = result.scalar_one_or_none()
    if not tx:
        raise HTTPException(404, "Transaction not found")
    return {"hash": tx.hash, "blockNumber": tx.block_number, "from": tx.from_address, "to": tx.to_address,
            "value": tx.value, "data": tx.data, "gasLimit": tx.gas_limit, "gasPrice": tx.gas_price,
            "status": tx.status, "timestamp": tx.timestamp.isoformat()}


# ─── Accounts ────────────────────────────────────


async def _seed_accounts(db: AsyncSession, count: int = 5):
    result = await db.execute(select(func.count()).select_from(Account))
    if result.scalar() > 0:
        return
    labels = ["Validator-1", "Alice", "Bob", "Treasury", "Swarm-Fund"]
    for i in range(count):
        db.add(Account(address=_a(), balance=str(random.randint(1000, 999999)),
                nonce=random.randint(0, 100), label=labels[i]))
    await db.commit()


@app.get("/api/accounts")
async def get_accounts(db: AsyncSession = Depends(get_session)):
    await _seed_accounts(db)
    result = await db.execute(select(Account))
    return [{"address": a.address, "publicKey": a.public_key, "keyType": a.key_type,
             "balance": a.balance, "nonce": a.nonce, "label": a.label, "network": a.network,
             "createdAt": a.created_at.isoformat()} for a in result.scalars().all()]


@app.get("/api/accounts/{address}")
async def get_account(address: str, db: AsyncSession = Depends(get_session)):
    result = await db.execute(select(Account).where(Account.address == address))
    account = result.scalar_one_or_none()
    if not account:
        raise HTTPException(404, "Account not found")
    return {"address": account.address, "publicKey": account.public_key, "keyType": account.key_type,
            "balance": account.balance, "nonce": account.nonce, "label": account.label, "network": account.network,
            "createdAt": account.created_at.isoformat()}


# ─── Contracts ────────────────────────────────────


async def _seed_contracts(db: AsyncSession, count: int = 3):
    result = await db.execute(select(func.count()).select_from(Contract))
    if result.scalar() > 0:
        return
    names = ["X3Token", "RouterV2", "StakingPool"]
    for i in range(count):
        db.add(Contract(address=_a(), name=names[i],
                abi='[{"type":"function","name":"balanceOf","inputs":[{"name":"owner","type":"address"}]}]',
                bytecode="0x" + "".join(random.choices("0123456789abcdef", k=4096)), owner=_a(),
                verified=random.choice([True, False])))
    await db.commit()


@app.get("/api/contracts")
async def get_contracts(db: AsyncSession = Depends(get_session)):
    await _seed_contracts(db)
    result = await db.execute(select(Contract))
    return [{"address": c.address, "name": c.name, "owner": c.owner, "verified": c.verified,
             "compiler": c.compiler, "sourcePath": c.source_path, "txHash": c.tx_hash,
             "deployedAt": c.deployed_at.isoformat()} for c in result.scalars().all()]


@app.get("/api/contracts/{address}")
async def get_contract(address: str, db: AsyncSession = Depends(get_session)):
    result = await db.execute(select(Contract).where(Contract.address == address))
    c = result.scalar_one_or_none()
    if not c:
        raise HTTPException(404, "Contract not found")
    return {"address": c.address, "name": c.name, "abi": c.abi, "bytecode": c.bytecode,
            "owner": c.owner, "verified": c.verified, "compiler": c.compiler,
            "sourcePath": c.source_path, "txHash": c.tx_hash,
            "deployedAt": c.deployed_at.isoformat()}


class DeployRequest(BaseModel):
    name: str
    abi: str = "[]"
    bytecode: str
    from_address: str
    gas_limit: int = 3000000
    gas_price: str = "100000000000"


@app.post("/api/contracts/deploy")
async def deploy_contract(req: DeployRequest, db: AsyncSession = Depends(get_session)):
    address = _a()
    tx_hash = _r()
    db.add(Contract(address=address, name=req.name, abi=req.abi, bytecode=req.bytecode,
            owner=req.from_address, verified=False, tx_hash=tx_hash))
    await db.commit()
    return {"address": address, "txHash": tx_hash, "name": req.name, "from": req.from_address}


# ─── Transaction Builder ─────────────────────────


class TxBuildRequest(BaseModel):
    from_address: str
    to: str = ""
    value: str = "0"
    data: str = "0x"
    gas_limit: int = 21000
    gas_price: str = "100000000000"
    nonce: int = 0


@app.post("/api/tx/build")
async def build_transaction(req: TxBuildRequest):
    return {
        "unsigned": {
            "from": req.from_address,
            "to": req.to,
            "value": req.value,
            "data": req.data,
            "gasLimit": hex(req.gas_limit),
            "gasPrice": hex(int(req.gas_price)),
            "nonce": hex(req.nonce),
            "chainId": hex(1),
        },
        "rlp": f"0x{'00' * 100}",
        "hash": _r(),
    }


@app.post("/api/tx/estimate")
async def estimate_gas(req: TxBuildRequest):
    try:
        async with httpx.AsyncClient(timeout=5) as client:
            r = await client.post(X3_RPC, json={
                "jsonrpc": "2.0", "id": 1, "method": "eth_estimateGas",
                "params": [{"from": req.from_address, "to": req.to, "data": req.data, "value": hex(int(req.value))}],
            })
            result = r.json().get("result", "0x0")
            return {"gasEstimate": int(result, 16) if isinstance(result, str) else 0, "raw": result}
    except Exception as e:
        return {"gasEstimate": 21000, "error": str(e)}


# ─── State Inspector ──────────────────────────────


@app.get("/api/inspect/balance")
async def inspect_balance(address: str = Query(...)):
    try:
        async with httpx.AsyncClient(timeout=5) as client:
            r = await client.post(X3_RPC, json={
                "jsonrpc": "2.0", "id": 1, "method": "eth_getBalance",
                "params": [address, "latest"],
            })
            result = r.json().get("result", "0x0")
            return {"address": address, "balance": str(int(result, 16)) if isinstance(result, str) else "0", "balanceHex": result}
    except Exception as e:
        return {"address": address, "balance": "0", "error": str(e)}


@app.get("/api/inspect/code")
async def inspect_code(address: str = Query(...)):
    try:
        async with httpx.AsyncClient(timeout=5) as client:
            r = await client.post(X3_RPC, json={
                "jsonrpc": "2.0", "id": 1, "method": "eth_getCode",
                "params": [address, "latest"],
            })
            result = r.json().get("result", "0x")
            return {"address": address, "code": result, "hasCode": result != "0x" and len(result) > 2}
    except Exception as e:
        return {"address": address, "code": "0x", "error": str(e)}


@app.get("/api/inspect/storage")
async def inspect_storage(address: str = Query(...), slot: str = Query("0x0")):
    try:
        async with httpx.AsyncClient(timeout=5) as client:
            r = await client.post(X3_RPC, json={
                "jsonrpc": "2.0", "id": 1, "method": "eth_getStorageAt",
                "params": [address, slot, "latest"],
            })
            result = r.json().get("result", "0x")
            return {"address": address, "slot": slot, "value": result}
    except Exception as e:
        return {"address": address, "slot": slot, "value": "0x", "error": str(e)}


# ─── Events ──────────────────────────────────────


class EventSubscription(BaseModel):
    address: Optional[str] = None
    topics: list[str] = []
    fromBlock: str = "0x0"
    toBlock: str = "latest"


@app.post("/api/events")
async def get_events(req: EventSubscription):
    params = {"fromBlock": req.fromBlock, "toBlock": req.toBlock, "address": req.address, "topics": req.topics} if req.address else {"fromBlock": req.fromBlock, "toBlock": req.toBlock}
    try:
        async with httpx.AsyncClient(timeout=5) as client:
            r = await client.post(X3_RPC, json={
                "jsonrpc": "2.0", "id": 1, "method": "eth_getLogs",
                "params": [params],
            })
            return r.json().get("result", [])
    except Exception as e:
        return {"error": str(e)}


# ─── Swarm Proxy ──────────────────────────────────


@app.get("/api/swarm/health")
async def swarm_health():
    try:
        async with httpx.AsyncClient(timeout=2) as client:
            r = await client.get(f"{SWARM_API}/health")
            return r.json()
    except Exception as e:
        return {"status": "unavailable", "error": str(e)}


# ─── Search ──────────────────────────────────────


@app.get("/api/search")
async def search(q: str = "", db: AsyncSession = Depends(get_session)):
    if not q:
        return {"blocks": [], "transactions": [], "accounts": [], "contracts": []}
    results: dict = {"blocks": [], "transactions": [], "accounts": [], "contracts": []}
    if q.startswith("0x") and len(q) == 66:
        tx_result = await db.execute(select(Transaction).where(Transaction.hash.ilike(f"%{q}%")))
        tx = tx_result.scalar_one_or_none()
        if tx:
            results["transactions"].append({"hash": tx.hash, "blockNumber": tx.block_number})
    if q.isdigit():
        block_result = await db.execute(select(Block).where(Block.number == int(q)))
        block = block_result.scalar_one_or_none()
        if block:
            results["blocks"].append({"number": block.number, "hash": block.hash})
    return results


# ─── WebSocket ────────────────────────────────────


connected_clients: set[WebSocket] = set()


@app.websocket("/api/ws")
async def websocket_endpoint(websocket: WebSocket):
    await websocket.accept()
    connected_clients.add(websocket)
    try:
        while True:
            data = await websocket.receive_text()
            for client in connected_clients.copy():
                try:
                    await client.send_text(f"echo: {data}")
                except Exception:
                    connected_clients.discard(client)
    except WebSocketDisconnect:
        connected_clients.discard(websocket)


if __name__ == "__main__":
    import uvicorn
    uvicorn.run("main:app", host="127.0.0.1", port=8765, reload=True)
