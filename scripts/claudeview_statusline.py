#!/usr/bin/env python3
import sys, json, time, pathlib

data = json.load(sys.stdin)
data["updatedAt"] = time.time()
cache = pathlib.Path.home() / ".claude" / "claudeview_cache.json"
cache.write_text(json.dumps(data))
