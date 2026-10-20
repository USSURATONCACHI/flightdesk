#!/usr/bin/env python3
"""Exercise the Flightdeck MCP server like an agent would. Usage: mcp_smoke.py [url] [out_dir]"""
import json, sys, time, base64, urllib.request

URL = sys.argv[1] if len(sys.argv) > 1 else "http://127.0.0.1:7788/mcp"
OUT = sys.argv[2] if len(sys.argv) > 2 else "/tmp"
_id = 0

def rpc(method, params=None, notify=False):
    global _id
    msg = {"jsonrpc": "2.0", "method": method}
    if params is not None:
        msg["params"] = params
    if not notify:
        _id += 1
        msg["id"] = _id
    req = urllib.request.Request(URL, data=json.dumps(msg).encode(), headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=200) as r:
        body = r.read()
        return json.loads(body) if body else None

def call(tool, **args):
    r = rpc("tools/call", {"name": tool, "arguments": args})
    res = r.get("result") or r
    err = res.get("isError")
    content = res.get("content", [])
    text = next((c["text"] for c in content if c.get("type") == "text"), "")
    img = next((c for c in content if c.get("type") == "image"), None)
    status = "ERR " if err else "ok  "
    print(f"{status}{tool}({json.dumps(args)[:80]}) -> {text[:160].replace(chr(10),' ')}{' +image' if img else ''}")
    if err:
        return None, text
    try:
        return json.loads(text), img
    except Exception:
        return text, img

init = rpc("initialize", {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "smoke", "version": "1"}})
print("initialize ->", init["result"]["serverInfo"], init["result"]["protocolVersion"])
rpc("notifications/initialized", notify=True)
tools = rpc("tools/list")["result"]["tools"]
print(f"tools/list -> {len(tools)} tools:", ", ".join(t["name"] for t in tools))

st, _ = call("engine_status")
assert st and st["connected"], "engine not connected"
defs, _ = call("list_definitions")
assert any(d["key"] == "order_process" for d in defs)
d, _ = call("get_definition", definition="order_process")
els, _ = call("list_elements", definition="order_process", type="task")
assert els["count"] >= 6
el, _ = call("get_element", definition="order_process", element_id="Task_Validate")
assert el["extensions"]["inputs"], "I/O mapping missing"
fe, _ = call("find_elements", definition="order_process", query="validateOrder")
assert fe["count"] == 1
sr, _ = call("search_instances", query="def:order_process var.amount>1000 state:any", page_size=10)
assert sr["count"] >= 3, sr
sr2, _ = call("search_instances", query="bk:ORD-2001")
pid = sr2["instances"][0]["id"]
inst, _ = call("get_instance", id=pid)
assert inst["tokens"][0]["activity_id"] == "Task_Approve"
vars_, _ = call("get_variables", id=pid)
hist, _ = call("get_activity_history", id=pid)
inc, _ = call("list_incidents", limit=5)
if inc:
    job = inc[0].get("configuration")
    call("get_error_details", job_id=job)
call("list_tasks", process_instance_id=pid)
call("list_jobs", process_instance_id=pid)
call("list_external_tasks", topic="stock")
call("list_batches")
# writes
r, _ = call("set_variable", id=pid, name="mcp_note", value={"by": "agent", "n": 1})
v2, _ = call("get_variables", id=pid, names=["mcp_note"])
assert v2 and v2[0]["type"] == "Json" and v2[0]["parsed"]["by"] == "agent"
_, e = call("delete_variable", id=pid, name="mcp_note")
assert e and "confirm" in e
call("delete_variable", id=pid, name="mcp_note", confirm=True)
# UI
call("ui_navigate", to=f"instance:{pid}")
time.sleep(1.5)
ui, _ = call("ui_state")
assert ui["place"]["kind"] == "instance", ui["place"]
time.sleep(1.0)
call("ui_select", element_id="Task_Approve")
call("ui_dock", tab="Variables")
call("ui_find", query="timer")
call("ui_zoom", level=1.2)
call("ui_commands")
call("ui_key", key="Escape")
call("ui_pan", dx=40, dy=0)
time.sleep(0.8)
shot, img = call("ui_screenshot", max_width=1400)
if img:
    open(f"{OUT}/mcp_shot_instance.png", "wb").write(base64.b64decode(img["data"]))
    print("saved", f"{OUT}/mcp_shot_instance.png", shot)
call("ui_navigate", to="def:order_process", new_tab=True)
time.sleep(1.5)
call("ui_find", query="approve")
call("ui_dock", tab="Instances")
call("ui_filter", text="ORD-2")
time.sleep(1.2)
shot, img = call("ui_screenshot", max_width=1400)
if img:
    open(f"{OUT}/mcp_shot_definition.png", "wb").write(base64.b64decode(img["data"]))
call("ui_navigate", to="search:bk:ORD-3*")
time.sleep(1.5)
ui, _ = call("ui_state")
print("search place:", ui["place"])
tabs, _ = call("ui_tabs", action="list")
call("ui_tabs", action="close", index=len(tabs["tabs"]) - 1)
# confirmation dialog round trip via UI: queue a modification and read it back
call("ui_navigate", to=f"instance:{pid}")
time.sleep(1.2)
call("ui_modification_queue", action="add", instruction={"type": "startBeforeActivity", "activityId": "Task_Notify"})
q, _ = call("ui_modification_queue", action="list")
assert len(q["instructions"]) == 1
call("ui_modification_queue", action="clear")
call("ui_settings", set={"theme": "light"})
time.sleep(0.5)
shot, img = call("ui_screenshot", max_width=1400)
if img:
    open(f"{OUT}/mcp_shot_light.png", "wb").write(base64.b64decode(img["data"]))
call("ui_settings", set={"theme": "dark"})
st, _ = call("engine_status")
print("MCP stats:", st["mcp"])
print("ALL GOOD")
