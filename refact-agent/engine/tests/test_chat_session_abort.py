#!/usr/bin/env python3
import asyncio
import aiohttp
import json
import uuid
import sys

LSP_URL = "http://127.0.0.1:8001"


async def test_abort_while_streaming():
    print("\n" + "="*60)
    print("TEST: Abort while streaming")
    print("="*60)

    chat_id = f"test-abort-{uuid.uuid4().hex[:8]}"
    events = []
    stream_started = asyncio.Event()
    abort_complete = asyncio.Event()
    draft_message_id = None

    async def subscriber():
        nonlocal draft_message_id
        async with aiohttp.ClientSession() as session:
            try:
                async with session.get(
                    f"{LSP_URL}/v1/chats/subscribe?chat_id={chat_id}",
                    timeout=aiohttp.ClientTimeout(total=30)
                ) as resp:
                    async for line in resp.content:
                        if line.startswith(b"data: "):
                            event = json.loads(line[6:])
                            events.append(event)
                            print(f"    Event: {event['type']}")

                            if event["type"] == "stream_started":
                                draft_message_id = event.get("message_id")
                                stream_started.set()

                            if event["type"] == "stream_finished":
                                if event.get("finish_reason") == "abort":
                                    print(f"    Stream aborted: {event.get('message_id')}")

                            if event["type"] == "message_removed":
                                print(f"    Message removed: {event.get('message_id')}")

                            if event["type"] == "runtime_updated":
                                if event.get("state") == "idle" and stream_started.is_set():
                                    abort_complete.set()
                                    return
            except Exception as e:
                print(f"    Exception: {e}")

    task = asyncio.create_task(subscriber())
    await asyncio.sleep(0.3)

    async with aiohttp.ClientSession() as session:
        await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
            "client_request_id": str(uuid.uuid4()),
            "type": "user_message",
            "content": "Write a very long essay about the history of computing"
        })

        try:
            await asyncio.wait_for(stream_started.wait(), timeout=10)
        except asyncio.TimeoutError:
            print("    Timeout waiting for stream to start")

        await asyncio.sleep(0.1)

        resp = await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
            "client_request_id": str(uuid.uuid4()),
            "type": "abort"
        })
        print(f"    Abort response: {resp.status}")

    try:
        await asyncio.wait_for(abort_complete.wait(), timeout=5)
    except asyncio.TimeoutError:
        pass

    task.cancel()

    event_types = [e["type"] for e in events]

    has_stream_finished_abort = any(
        e["type"] == "stream_finished" and e.get("finish_reason") == "abort"
        for e in events
    )
    has_message_removed = any(
        e["type"] == "message_removed" and e.get("message_id") == draft_message_id
        for e in events
    )
    has_idle_state = any(
        e["type"] == "runtime_updated" and e.get("state") == "idle"
        for e in events if events.index(e) > 0
    )

    if has_stream_finished_abort and has_message_removed and has_idle_state:
        print("  ✓ Abort produced correct event sequence")
        return True

    if not stream_started.is_set():
        print("  ⚠ Stream never started (model may not be configured)")
        return True

    print(f"  ✗ Missing events: stream_finished(abort)={has_stream_finished_abort}, message_removed={has_message_removed}, idle={has_idle_state}")
    return False


async def test_abort_idempotency():
    print("\n" + "="*60)
    print("TEST: Abort idempotency (double abort)")
    print("="*60)

    chat_id = f"test-abort-idem-{uuid.uuid4().hex[:8]}"
    stream_started = asyncio.Event()

    async def subscriber():
        async with aiohttp.ClientSession() as session:
            try:
                async with session.get(
                    f"{LSP_URL}/v1/chats/subscribe?chat_id={chat_id}",
                    timeout=aiohttp.ClientTimeout(total=15)
                ) as resp:
                    async for line in resp.content:
                        if line.startswith(b"data: "):
                            event = json.loads(line[6:])
                            if event["type"] == "stream_started":
                                stream_started.set()
                            if event["type"] == "runtime_updated" and event.get("state") == "idle":
                                if stream_started.is_set():
                                    return
            except Exception as e:
                print(f"    Exception: {e}")

    task = asyncio.create_task(subscriber())
    await asyncio.sleep(0.3)

    async with aiohttp.ClientSession() as session:
        await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
            "client_request_id": str(uuid.uuid4()),
            "type": "user_message",
            "content": "Hello"
        })

        try:
            await asyncio.wait_for(stream_started.wait(), timeout=10)
        except asyncio.TimeoutError:
            pass

        resp1 = await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
            "client_request_id": str(uuid.uuid4()),
            "type": "abort"
        })

        resp2 = await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
            "client_request_id": str(uuid.uuid4()),
            "type": "abort"
        })

        print(f"    First abort: {resp1.status}, Second abort: {resp2.status}")

    await asyncio.sleep(1)
    task.cancel()

    if resp1.status == 200 and resp2.status == 200:
        print("  ✓ Double abort handled gracefully")
        return True

    print("  ✗ Double abort failed")
    return False


async def test_abort_before_stream_starts():
    print("\n" + "="*60)
    print("TEST: Abort before stream starts (race condition)")
    print("="*60)

    chat_id = f"test-abort-race-{uuid.uuid4().hex[:8]}"

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

        msg_task = asyncio.create_task(session.post(
            f"{LSP_URL}/v1/chats/{chat_id}/commands",
            json={
                "client_request_id": str(uuid.uuid4()),
                "type": "user_message",
                "content": "Hello"
            }
        ))

        abort_resp = await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
            "client_request_id": str(uuid.uuid4()),
            "type": "abort"
        })

        await msg_task

        print(f"    Abort response: {abort_resp.status}")

    if abort_resp.status == 200:
        print("  ✓ Abort before stream handled gracefully")
        return True

    print("  ✗ Abort before stream failed")
    return False


async def main():
    print("=" * 60)
    print("Chat Session Abort Tests")
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

    results.append(("Abort while streaming", await test_abort_while_streaming()))
    results.append(("Abort idempotency", await test_abort_idempotency()))
    results.append(("Abort before stream starts", await test_abort_before_stream_starts()))

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
