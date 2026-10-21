#!/usr/bin/env python3
"""Record the README demo: drive a running Flightdeck through its MCP server, capture frames
with ui_screenshot, and assemble docs/demo.gif with ffmpeg (palette-optimised, GitHub-friendly).

Usage: scripts/demo.py [mcp_url] [out.gif]   (default http://127.0.0.1:7797/mcp, docs/demo.gif)
Launch the app first, e.g. with scripts/shot.sh-style env and
  flightdeck --size 1280x800 --mcp --mcp-port 7797 --url http://127.0.0.1:8089/engine-rest
The scenario: open the process, zoom into the diagram, search a business key, jump to the
hit, inspect the task, open the instance, look at variables, switch to Russian, back."""
import base64, json, os, shutil, subprocess, sys, tempfile, time, urllib.request

URL = sys.argv[1] if len(sys.argv) > 1 else "http://127.0.0.1:7797/mcp"
OUT = sys.argv[2] if len(sys.argv) > 2 else "docs/demo.gif"
FPS = 8  # frames per second in the GIF; holds are expressed as repeated frames

def call(tool, **args):
    body = json.dumps({"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": tool, "arguments": args}}).encode()
    req = urllib.request.Request(URL, body, {"content-type": "application/json", "accept": "application/json"})
    r = json.loads(urllib.request.urlopen(req, timeout=60).read())
    res = r.get("result") or r
    content = res.get("content", [])
    text = next((c["text"] for c in content if c.get("type") == "text"), "")
    img = next((c for c in content if c.get("type") == "image"), None)
    return text, img

frames = []
tmp = tempfile.mkdtemp()

def shot(hold=1.0):
    """Capture one frame and hold it for `hold` seconds."""
    _, img = call("ui_screenshot", max_width=1280)
    data = base64.b64decode(img["data"])
    p = os.path.join(tmp, f"f{len(frames):04d}.png")
    open(p, "wb").write(data)
    for _ in range(max(1, round(hold * FPS))):
        frames.append(p)

def animate(seconds, fps=FPS):
    """Capture live frames for `seconds` (camera glides, fades)."""
    end = time.time() + seconds
    while time.time() < end:
        t0 = time.time()
        shot(1.0 / fps)
        time.sleep(max(0.0, 1.0 / fps - (time.time() - t0)))

# --- scenario ---
call("ui_navigate", to="definitions"); time.sleep(1.5); shot(1.5)
call("ui_navigate", to="def:order_process"); time.sleep(2.0); shot(1.5)
call("ui_zoom", level=1.6); animate(1.2); shot(0.8)
call("ui_find", query="ORD-1001"); time.sleep(2.0); shot(2.0)            # business-key hit, teal ring, legend
call("ui_zoom", level=2.6); animate(1.2); shot(1.5)                       # off-screen → edge arrow + minimap dot
call("ui_find", goto=0); animate(1.4); shot(1.5)                          # glide to the hit
call("ui_find", query=""); call("ui_select", element_id="Task_Approve"); call("ui_dock", tab="Inspector"); time.sleep(1.0); shot(2.5)
rows, _ = call("search_instances", query="def:order_process bk:ORD-1001", page_size=1)
inst = (json.loads(rows).get("instances") or [{}])[0].get("id")
if inst:
    call("ui_navigate", to=f"instance:{inst}"); time.sleep(3.0); shot(2.5)   # tokens, history
    call("ui_dock", tab="Variables"); time.sleep(1.0); shot(2.0)
    call("ui_dock", tab="Timeline"); time.sleep(1.0); shot(2.0)
call("ui_settings", set={"language": "ru-RU"}); time.sleep(0.8); shot(2.0)
call("ui_key", key="F1"); time.sleep(0.8); shot(2.0)
call("ui_key", key="Escape"); call("ui_settings", set={"language": "system"}); time.sleep(0.5); shot(1.0)

# --- assemble ---
lst = os.path.join(tmp, "frames.txt")
with open(lst, "w") as f:
    for p in frames:
        f.write(f"file '{p}'\nduration {1.0 / FPS}\n")
    f.write(f"file '{frames[-1]}'\n")
palette = os.path.join(tmp, "palette.png")
vf = "fps=%d,scale=1000:-1:flags=lanczos" % FPS
subprocess.run(["ffmpeg", "-y", "-loglevel", "error", "-f", "concat", "-safe", "0", "-i", lst, "-vf", vf + ",palettegen=max_colors=192:stats_mode=diff", palette], check=True)
subprocess.run(["ffmpeg", "-y", "-loglevel", "error", "-f", "concat", "-safe", "0", "-i", lst, "-i", palette, "-lavfi", vf + "[x];[x][1:v]paletteuse=dither=bayer:bayer_scale=4:diff_mode=rectangle", "-loop", "0", OUT], check=True)
shutil.rmtree(tmp)
print(f"{OUT}: {os.path.getsize(OUT) / 1e6:.1f} MB, {len(frames)} frames, {len(frames) / FPS:.0f} s")
