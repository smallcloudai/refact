#!/usr/bin/env python3
"""
Tests for thread parameter changes via SSE events.

Run with:
  python tests/test_chat_session_thread_params.py

Requires:
  - refact-lsp running on port 8001
"""

import asyncio
import aiohttp
import json
import uuid
import sys

LSP_URL = "http://127.0.0.1:8001"


async def test_set_params_emits_thread_updated():
    """Test that set_params emits thread_updated event."""
    print("\n" + "="*60)
    print("TEST: set_params emits thread_updated")
    print("="*60)

    chat_id = f"test-params-{uuid.uuid4().hex[:8]}"
    events = []

    async def subscriber():
        async with aiohttp.ClientSession() as session:
            try:
                async with session.get(
                    f"{LSP_URL}/v1/chats/subscribe?chat_id={chat_id}",
                    timeout=aiohttp.ClientTimeout(total=10)
                ) as resp:
                    async for line in resp.content:
                        if line.startswith(b"data: "):
                            event = json.loads(line[6:])
                            events.append(event)
                            print(f"    Event: {event['type']}")

                            if event["type"] == "thread_updated":
                                print(f"    Params: {event}")
                                return
            except Exception as e:
                print(f"    Exception: {e}")

    task = asyncio.create_task(subscriber())
    await asyncio.sleep(0.3)

    async with aiohttp.ClientSession() as session:
        await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
            "client_request_id": str(uuid.uuid4()),
            "type": "set_params",
            "patch": {
                "model": "gpt-4o-mini",
                "mode": "NO_TOOLS",
                "boost_reasoning": True
            }
        })

    await asyncio.sleep(2)
    task.cancel()

    event_types = [e["type"] for e in events]
    print(f"\n  Event sequence: {event_types}")

    thread_updated_events = [e for e in events if e["type"] == "thread_updated"]
    if thread_updated_events:
        event = thread_updated_events[0]
        checks = []
        if event.get("model") == "gpt-4o-mini":
            checks.append("model")
        if event.get("mode") == "NO_TOOLS":
            checks.append("mode")
        if event.get("boost_reasoning") == True:
            checks.append("boost_reasoning")

        if checks:
            print(f"  ✓ thread_updated received with: {', '.join(checks)}")
            return True

    print("  ✗ thread_updated event not received or missing params")
    return False


async def test_title_update_emits_title_updated():
    """Test that setting title emits title_updated event."""
    print("\n" + "="*60)
    print("TEST: set_params with title emits title_updated")
    print("="*60)

    chat_id = f"test-title-{uuid.uuid4().hex[:8]}"
    events = []

    async def subscriber():
        async with aiohttp.ClientSession() as session:
            try:
                async with session.get(
                    f"{LSP_URL}/v1/chats/subscribe?chat_id={chat_id}",
                    timeout=aiohttp.ClientTimeout(total=10)
                ) as resp:
                    async for line in resp.content:
                        if line.startswith(b"data: "):
                            event = json.loads(line[6:])
                            events.append(event)
                            print(f"    Event: {event['type']}")

                            if event["type"] == "title_updated":
                                print(f"    Title: {event.get('title')}")
                                return
            except Exception as e:
                print(f"    Exception: {e}")

    task = asyncio.create_task(subscriber())
    await asyncio.sleep(0.3)

    async with aiohttp.ClientSession() as session:
        await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
            "client_request_id": str(uuid.uuid4()),
            "type": "set_params",
            "patch": {
                "title": "My Custom Title"
            }
        })

    await asyncio.sleep(2)
    task.cancel()

    event_types = [e["type"] for e in events]
    print(f"\n  Event sequence: {event_types}")

    title_events = [e for e in events if e["type"] == "title_updated"]
    if title_events:
        if title_events[0].get("title") == "My Custom Title":
            print("  ✓ title_updated received with correct title")
            return True

    thread_updated = [e for e in events if e["type"] == "thread_updated" and e.get("title")]
    if thread_updated:
        if thread_updated[0].get("title") == "My Custom Title":
            print("  ✓ title in thread_updated (alternative)")
            return True

    print("  ✗ title_updated event not received")
    return False


async def test_snapshot_reflects_params():
    """Test that snapshot after reconnect reflects updated params."""
    print("\n" + "="*60)
    print("TEST: Snapshot reflects updated params after reconnect")
    print("="*60)

    chat_id = f"test-snapshot-params-{uuid.uuid4().hex[:8]}"

    async with aiohttp.ClientSession() as session:
        async with session.get(
            f"{LSP_URL}/v1/chats/subscribe?chat_id={chat_id}",
            timeout=aiohttp.ClientTimeout(total=5)
        ) as resp:
            async for line in resp.content:
                if line.startswith(b"data: "):
                    event = json.loads(line[6:])
                    if event["type"] == "snapshot":
                        break

        await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
            "client_request_id": str(uuid.uuid4()),
            "type": "set_params",
            "patch": {
                "model": "test-model-xyz",
                "mode": "EXPLORE",
                "boost_reasoning": True,
                "checkpoints_enabled": False,
                "include_project_info": False
            }
        })
        await asyncio.sleep(0.5)

        async with session.get(
            f"{LSP_URL}/v1/chats/subscribe?chat_id={chat_id}",
            timeout=aiohttp.ClientTimeout(total=5)
        ) as resp:
            async for line in resp.content:
                if line.startswith(b"data: "):
                    event = json.loads(line[6:])
                    if event["type"] == "snapshot":
                        thread = event.get("thread", {})
                        checks = []

                        if thread.get("model") == "test-model-xyz":
                            checks.append("model")
                        if thread.get("mode") == "EXPLORE":
                            checks.append("mode")
                        if thread.get("boost_reasoning") == True:
                            checks.append("boost_reasoning")
                        if thread.get("checkpoints_enabled") == False:
                            checks.append("checkpoints_enabled")
                        if thread.get("include_project_info") == False:
                            checks.append("include_project_info")

                        print(f"  Thread in snapshot: {thread}")
                        print(f"  Verified params: {checks}")

                        if len(checks) >= 3:
                            print("  ✓ Snapshot reflects updated params")
                            return True
                        break

    print("  ✗ Snapshot does not reflect updated params")
    return False


async def test_multiple_param_updates():
    """Test that multiple param updates are all reflected."""
    print("\n" + "="*60)
    print("TEST: Multiple param updates all reflected")
    print("="*60)

    chat_id = f"test-multi-params-{uuid.uuid4().hex[:8]}"
    thread_updated_count = 0

    async def subscriber():
        nonlocal thread_updated_count
        async with aiohttp.ClientSession() as session:
            try:
                async with session.get(
                    f"{LSP_URL}/v1/chats/subscribe?chat_id={chat_id}",
                    timeout=aiohttp.ClientTimeout(total=10)
                ) as resp:
                    async for line in resp.content:
                        if line.startswith(b"data: "):
                            event = json.loads(line[6:])
                            if event["type"] == "thread_updated":
                                thread_updated_count += 1
                                print(f"    thread_updated #{thread_updated_count}")
                            if thread_updated_count >= 3:
                                return
            except Exception as e:
                print(f"    Exception: {e}")

    task = asyncio.create_task(subscriber())
    await asyncio.sleep(0.3)

    async with aiohttp.ClientSession() as session:
        await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
            "client_request_id": str(uuid.uuid4()),
            "type": "set_params",
            "patch": {"model": "model-1"}
        })
        await asyncio.sleep(0.2)

        await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
            "client_request_id": str(uuid.uuid4()),
            "type": "set_params",
            "patch": {"mode": "AGENT"}
        })
        await asyncio.sleep(0.2)

        await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
            "client_request_id": str(uuid.uuid4()),
            "type": "set_params",
            "patch": {"boost_reasoning": True}
        })

    await asyncio.sleep(2)
    task.cancel()

    print(f"\n  Total thread_updated events: {thread_updated_count}")

    if thread_updated_count >= 3:
        print("  ✓ All param updates emitted thread_updated")
        return True

    print("  ✗ Not all param updates received")
    return False


async def main():
    print("=" * 60)
    print("Chat Session Thread Params Tests")
    print("=" * 60)
    print(f"Testing against: {LSP_URL}")

    try:
        async with aiohttp.ClientSession() as session:
            async with session.get(f"{LSP_URL}/v1/ping", timeout=aiohttp.ClientTimeout(total=2)) as resp:
                if resp.status != 200:
                    print(f"\n✗ Server not responding correctly at {LSP_URL}")
                    sys.exit(1)
    except Exception as e:
        print(f"\n✗ Cannot connect to server at {LSP_URL}: {e}")
        sys.exit(1)

    print("✓ Server is running\n")

    results = []

    results.append(("set_params emits thread_updated", await test_set_params_emits_thread_updated()))
    results.append(("title update emits title_updated", await test_title_update_emits_title_updated()))
    results.append(("Snapshot reflects params", await test_snapshot_reflects_params()))
    results.append(("Multiple param updates", await test_multiple_param_updates()))

    print("\n" + "=" * 60)
    print("Summary")
    print("=" * 60)

    passed = sum(1 for _, r in results if r)
    total = len(results)

    for name, result in results:
        status = "✓ PASS" if result else "✗ FAIL"
        print(f"  {status}: {name}")

    print(f"\nTotal: {passed}/{total} passed")

    sys.exit(0 if passed == total else 1)


if __name__ == "__main__":
    asyncio.run(main())
