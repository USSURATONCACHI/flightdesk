#!/usr/bin/env python3
"""Host file server for the test VM: serves the current directory (GET) and accepts uploads
from the guest (POST /upload/<name> saves the body into uploads/<name>). Usage: serve.py [port]"""
import http.server, os, sys

class Handler(http.server.SimpleHTTPRequestHandler):
    # Extension-less control files (msi-name, zip-name, next) must arrive as text, or
    # PowerShell's Invoke-WebRequest hands the script a byte[] instead of a string.
    def guess_type(self, path):
        return "text/plain; charset=utf-8" if "." not in os.path.basename(path) else super().guess_type(path)

    def do_POST(self):
        if not self.path.startswith("/upload/"):
            self.send_error(404)
            return
        name = os.path.basename(self.path[len("/upload/"):]) or "upload.bin"
        n = int(self.headers.get("Content-Length") or 0)
        os.makedirs("uploads", exist_ok=True)
        with open(os.path.join("uploads", name), "wb") as f:
            f.write(self.rfile.read(n))
        self.send_response(200)
        self.send_header("Content-Length", "2")
        self.end_headers()
        self.wfile.write(b"ok")
        self.log_message("uploaded %s (%d bytes)", name, n)

    def log_message(self, fmt, *args):
        sys.stderr.write("%s %s\n" % (self.log_date_time_string(), fmt % args))
        sys.stderr.flush()

port = int(sys.argv[1]) if len(sys.argv) > 1 else 8000
http.server.ThreadingHTTPServer(("127.0.0.1", port), Handler).serve_forever()
