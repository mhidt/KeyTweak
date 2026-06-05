import sys
import json
import time
import asyncio
import threading
import os

MOCK_TRANSLATIONS = {
    ("en", "ru"): {"hello": "Привет", "world": "мир", "test": "тест"},
    ("ru", "en"): {"привет": "Hello", "мир": "world", "тест": "test"},
}

DELAY = float(os.environ.get("MOCK_DELAY", "0.1"))
FAIL_AFTER = int(os.environ.get("MOCK_FAIL_AFTER", "0"))
FAIL_NEVER = os.environ.get("MOCK_FAIL_NEVER", "1") == "1"

_send_lock = asyncio.Lock()
_request_count = 0

async def send(obj):
    async with _send_lock:
        sys.stdout.write(json.dumps(obj, ensure_ascii=False) + "\n")
        sys.stdout.flush()

async def heartbeat_loop():
    while True:
        await send({"type": "heartbeat", "timestamp": time.time()})
        await asyncio.sleep(30)

async def handle_translate(req):
    global _request_count
    _request_count += 1

    if not FAIL_NEVER and FAIL_AFTER > 0 and _request_count > FAIL_AFTER:
        await send({"id": req["id"], "error": {"code": "TRANSLATION_ERROR", "message": "mock crash", "recoverable": True}})
        return

    await asyncio.sleep(DELAY)
    key = (req["source"], req["target"])
    text_lower = req["q"].lower().strip()
    translations = MOCK_TRANSLATIONS.get(key, {})
    translated = translations.get(text_lower, f"[{req['target']}] {req['q']}")
    await send({"id": req["id"], "translated": translated})

def _stdin_reader(queue):
    try:
        for line in sys.stdin:
            line = line.strip()
            if line:
                queue.put_nowait(line)
    except Exception:
        pass
    finally:
        queue.put_nowait(None)

async def _read_lines(queue):
    while True:
        while queue.empty():
            await asyncio.sleep(0.01)
        item = queue.get_nowait()
        if item is None:
            return
        yield item

async def main():
    sys.stderr.reconfigure(line_buffering=True)
    sys.stdout.reconfigure(encoding="utf-8")

    queue = asyncio.Queue()
    thread = threading.Thread(target=_stdin_reader, args=(queue,), daemon=True)
    thread.start()

    init_received = False
    heartbeat_task = None

    async for line in _read_lines(queue):
        try:
            req = json.loads(line)
        except json.JSONDecodeError:
            continue

        cmd = req.get("cmd")
        if cmd == "init":
            await send({"type": "init", "protocol_version": "1.0", "ready": True, "capabilities": ["translate", "status"]})
            if heartbeat_task is None:
                heartbeat_task = asyncio.create_task(heartbeat_loop())
            init_received = True
        elif cmd == "translate":
            if init_received:
                asyncio.create_task(handle_translate(req))
        elif cmd == "status":
            if init_received:
                await send({"id": req.get("id"), "ready": True, "languages": ["en", "ru"]})
        elif cmd == "exit":
            await send({"type": "shutdown", "reason": req.get("reason", "graceful")})
            break

if __name__ == "__main__":
    asyncio.run(main())
