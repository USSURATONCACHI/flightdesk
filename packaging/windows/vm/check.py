#!/usr/bin/env python3
"""Exercise Flightdeck running inside the Windows VM through its MCP server (forwarded to
127.0.0.1:7799): engine connection, places, find, zoom/keys, settings, language switch,
screenshots. Prints a PASS/FAIL line per check and saves screenshots next to this script's
output dir. Usage: check.py [out_dir]"""
import base64, json, os, sys, time, urllib.request

URL = "http://127.0.0.1:7799/mcp"
OUT = sys.argv[1] if len(sys.argv) > 1 else os.path.expanduser("~/.cache/flightdeck-vm/out")
os.makedirs(OUT, exist_ok=True)
results = []

def rpc(method, params=None, timeout=60):
    body = json.dumps({"jsonrpc": "2.0", "id": 1, "method": method, "params": params or {}}).encode()
    req = urllib.request.Request(URL, body, {"content-type": "application/json", "accept": "application/json"})
    return json.loads(urllib.request.urlopen(req, timeout=timeout).read())

def call(tool, **args):
    r = rpc("tools/call", {"name": tool, "arguments": args})
    res = r.get("result") or r
    content = res.get("content", [])
    text = next((c["text"] for c in content if c.get("type") == "text"), "")
    img = next((c for c in content if c.get("type") == "image"), None)
    if res.get("isError"):
        raise RuntimeError(f"{tool}: {text}")
    try:
        return json.loads(text), img
    except Exception:
        return text, img

def check(name, cond, detail=""):
    results.append((name, bool(cond), detail))
    print(("PASS " if cond else "FAIL ") + name + (f"  — {detail}" if detail else ""), flush=True)

def shot(name):
    _, img = call("ui_screenshot", max_width=1400)
    if img:
        p = os.path.join(OUT, name)
        open(p, "wb").write(base64.b64decode(img["data"]))
        return p
    return None

# 1. reachability
deadline = time.time() + float(os.environ.get("CHECK_WAIT", "900"))
tools = None
while time.time() < deadline:
    try:
        tools = rpc("tools/list", timeout=10)["result"]["tools"]
        break
    except Exception:
        time.sleep(5)
check("MCP reachable through the VM port forward", tools is not None, f"{len(tools or [])} tools")
if tools is None:
    sys.exit(1)

# 2. engine + place
st, _ = call("ui_state")
eng, _ = call("engine_status")
check("connected to the host's scratch engine", st.get("connected") and str(eng.get("version", "")).startswith("7."), json.dumps(eng)[:120])
check("definition place opened from the command line", st.get("place", {}).get("kind") == "definition" and st["place"].get("key") == "order_process", st.get("place", {}).get("kind"))
time.sleep(2)
st, _ = call("ui_state")
check("diagram loaded and rendered", st["place"].get("diagram", {}).get("loaded") is True, f"{st['place'].get('diagram', {}).get('elements')} elements")
check("running badges from the engine", sum((st["place"].get("running_by_activity") or {}).values()) >= 2, json.dumps(st["place"].get("running_by_activity")))
shot("01-definition.png")

# 3. find (elements + instances by business key), edge arrows, goto
f, _ = call("ui_find", query="ORD-100")
time.sleep(2.5)
st, _ = call("ui_state")
find = st["place"]["diagram"].get("find") or {}
check("find reports business-key instance hits", len(find.get("instance_hits", [])) >= 2, json.dumps(find.get("instance_hits"))[:160])
shot("02-find.png")
call("ui_zoom", level=2.5)
time.sleep(1.5)
st, _ = call("ui_state")
find = st["place"]["diagram"].get("find") or {}
off = [t for t in find.get("targets", []) if not t["visible"]]
check("off-screen hits detected after zoom", len(off) >= 1, f"{len(off)} off-screen, edge_arrows={find.get('edge_arrows')}")
shot("03-arrows.png")
if off:
    g, _ = call("ui_find", goto=off[0]["index"])
    time.sleep(1.5)
    st, _ = call("ui_state")
    vis = [t["visible"] for t in st["place"]["diagram"]["find"]["targets"] if t["index"] == off[0]["index"]]
    check("goto glides the camera to the hit", vis == [True], g.get("went_to"))
call("ui_find", query="")

# 4. keyboard shortcuts via synthetic keys
z0 = call("ui_state")[0]["place"]["diagram"]["zoom"]
call("ui_key", key="Plus")
time.sleep(1.0)
z1 = call("ui_state")[0]["place"]["diagram"]["zoom"]
check("keyboard zoom works", z1 > z0 * 1.3, f"{z0:.3f} -> {z1:.3f}")
call("ui_key", key="Num0")
time.sleep(1.0)

# 5. inspector + copy button path (the release hang)
call("ui_select", element_id="Task_Approve")
call("ui_dock", tab="Inspector")
time.sleep(1.0)
st, _ = call("ui_state")
check("selection + inspector dock", st["place"]["diagram"].get("selection", {}).get("element_id") == "Task_Approve")
shot("04-inspector.png")

# 6. settings: language switch round trip, zoom speed, theme
s, _ = call("ui_settings")
check("language packs found next to the exe", {p["tag"] for p in s.get("languages_available", [])} >= {"en-US", "ru-RU", "de-DE"}, json.dumps(s.get("languages_available")))
call("ui_settings", set={"language": "ru-RU"})
call("ui_navigate", to="settings", new_tab=True)
time.sleep(1.5)
s, _ = call("ui_settings")
check("switched UI language to Russian", s.get("language_active") == "ru-RU")
shot("05-settings-ru.png")
call("ui_settings", set={"language": "system", "theme": "light"})
time.sleep(1.0)
shot("06-settings-light.png")
call("ui_settings", set={"theme": "system"})
call("ui_tabs", action="close")

# 7. instance place end to end
inst, _ = call("search_instances", query="def:order_process state:active", page_size=5)
rows = inst.get("instances") or inst.get("rows") or []
check("instance search returns the seeded instances", len(rows) >= 2, f"{len(rows)} rows")
if rows:
    call("ui_navigate", to=f"instance:{rows[0]['id']}")
    time.sleep(3.0)
    st, _ = call("ui_state")
    check("instance place loads tokens", st["place"].get("kind") == "instance" and st["place"].get("diagram", {}).get("loaded"), json.dumps(st["place"].get("tokens", ""))[:120])
    shot("07-instance.png")

# 8. help overlay + responsiveness
call("ui_key", key="F1")
time.sleep(1.0)
shot("08-help.png")
call("ui_key", key="Escape")
st, _ = call("ui_state")
check("still responsive after the tour", isinstance(st.get("frame"), int))

ok = all(r[1] for r in results)
print(f"\n{sum(r[1] for r in results)}/{len(results)} checks passed; screenshots in {OUT}")
sys.exit(0 if ok else 2)
