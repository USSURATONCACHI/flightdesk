#!/usr/bin/env python3
"""Tiny MCP client for CI smoke tests: `mcp.py <port> wait <seconds>` waits for the server,
`mcp.py <port> call <tool> [json-args]` prints the tool's text result (exit 1 on tool error)."""
import json, sys, time, urllib.request

port, action = sys.argv[1], sys.argv[2]
url = f"http://127.0.0.1:{port}/mcp"

def rpc(method, params=None, timeout=10):
    body = json.dumps({"jsonrpc": "2.0", "id": 1, "method": method, "params": params or {}}).encode()
    req = urllib.request.Request(url, body, {"content-type": "application/json", "accept": "application/json"})
    return json.loads(urllib.request.urlopen(req, timeout=timeout).read())

if action == "wait":
    deadline = time.time() + float(sys.argv[3])
    while time.time() < deadline:
        try:
            n = len(rpc("tools/list")["result"]["tools"])
            print(f"MCP up with {n} tools")
            sys.exit(0)
        except Exception:
            time.sleep(2)
    sys.exit("MCP server did not come up")
elif action == "call":
    args = json.loads(sys.argv[4]) if len(sys.argv) > 4 else {}
    res = rpc("tools/call", {"name": sys.argv[3], "arguments": args}, timeout=60)["result"]
    print(next((c["text"] for c in res.get("content", []) if c.get("type") == "text"), ""))
    sys.exit(1 if res.get("isError") else 0)
