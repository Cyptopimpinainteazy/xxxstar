#!/usr/bin/env python3
"""Raw WS JSON-RPC probe against an X3 node (no fake adapter; real wire calls)."""
import asyncio, json, sys
import websockets

WS = sys.argv[1] if len(sys.argv) > 1 else "ws://127.0.0.1:9944"

async def call(ws, id, method, params):
    await ws.send(json.dumps({"jsonrpc":"2.0","id":id,"method":method,"params":params}))
    while True:
        msg = json.loads(await asyncio.wait_for(ws.recv(), 10))
        if msg.get("id") == id:
            return msg

async def main():
    async with websockets.connect(WS, open_timeout=8, max_size=50_000_000) as ws:
        props = await call(ws, 1, "system_properties", [])
        health = await call(ws, 2, "system_health", [])
        chain  = await call(ws, 3, "system_chain", [])
        name   = await call(ws, 4, "system_name", [])
        ver    = await call(ws, 5, "system_version", [])
        header = await call(ws, 6, "chain_getHeader", [])
        print("system_properties:", json.dumps(props.get("result"), indent=2))
        print("system_health:", json.dumps(health.get("result")))
        print("system_chain:", chain.get("result"))
        print("system_name:", name.get("result"))
        print("system_version:", ver.get("result"))
        h = header.get("result") or {}
        print("head number:", h.get("number"), "hash:", (h.get("hash") or "")[:16])
        # metadata version / runtime version via state_getRuntimeVersion
        rv = await call(ws, 7, "state_getRuntimeVersion", [])
        print("runtime version:", json.dumps(rv.get("result")))

asyncio.run(main())
