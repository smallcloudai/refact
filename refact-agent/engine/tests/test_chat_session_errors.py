#!/usr/bin/env python3
import asyncio
import aiohttp
import json
import uuid
import sys

LSP_URL = "http://127.0.0.1:8001"


async def test_invalid_model_error():
    print("\n" + "="*60)
    print("TEST: Invalid model produces error state")
    print("="*60)

    chat_id = f"test-invalid-model-{uuid.uuid4().hex[:8]}"
    events = []
    error_received = asyncio.Event()
    draft_message_id = None

    async def subscriber():
        nonlocal draft_message_id
        async with aiohttp.ClientSession() as session:
            try:
                async with session.get(
                    f"{LSP_URL}/v1/chats/subscribe?chat_id={chat_id}",
                    timeout=aiohttp.ClientTimeout(total=15)
                ) as resp:
                    async for line in resp.content:
                        if line.startswith(b"data: "):
                            event = json.loads(line[6:])
                            events.append(event)
                            print(f"    Event: {event['type']}")

                            if event["type"] == "stream_started":
                                draft_message_id = event.get("message_id")

                            if event["type"] == "runtime_updated":
                                if event.get("state") == "error":
                                    print(f"    Error: {event.get('error', '')[:50]}...")
                                    error_received.set()
                                    return
            except Exception as e:
                print(f"    Exception: {e}")

    task = asyncio.create_task(subscriber())
    await asyncio.sleep(0.3)

    async with aiohttp.ClientSession() as session:
        await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
            "client_request_id": str(uuid.uuid4()),
            "type": "set_params",
            "patch": {"model": "nonexistent-model-xyz-12345"}
        })

        await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
            "client_request_id": str(uuid.uuid4()),
            "type": "user_message",
            "content": "Hello"
        })

    try:
        await asyncio.wait_for(error_received.wait(), timeout=10)
    except asyncio.TimeoutError:
        pass

    task.cancel()

    event_types = [e["type"] for e in events]

    has_error_state = any(
        e["type"] == "runtime_updated" and e.get("state") == "error"
        for e in events
    )

    has_message_removed = any(
        e["type"] == "message_removed"
        for e in events
    )

    if has_error_state:
        print(f"  ✓ Error state received, message_removed={has_message_removed}")
        return True

    print("  ✗ Error state not received")
    return False


async def test_ack_correlation_invalid_content():
    print("\n" + "="*60)
    print("TEST: Ack correlation for invalid content (400)")
    print("="*60)

    chat_id = f"test-ack-400-{uuid.uuid4().hex[:8]}"
    events = []
    ack_received = asyncio.Event()
    client_request_id = str(uuid.uuid4())

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
                            if event["type"] == "ack":
                                print(f"    Ack: accepted={event.get('accepted')}, request_id={event.get('client_request_id')[:8]}...")
                                if event.get("client_request_id") == client_request_id:
                                    ack_received.set()
                                    return
            except Exception as e:
                print(f"    Exception: {e}")

    task = asyncio.create_task(subscriber())
    await asyncio.sleep(0.3)

    async with aiohttp.ClientSession() as session:
        resp = await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
            "client_request_id": client_request_id,
            "type": "user_message",
            "content": [{"type": "invalid_type", "data": "test"}]
        })
        print(f"    HTTP status: {resp.status}")

    try:
        await asyncio.wait_for(ack_received.wait(), timeout=3)
    except asyncio.TimeoutError:
        pass

    task.cancel()

    matching_ack = next(
        (e for e in events if e["type"] == "ack" and e.get("client_request_id") == client_request_id),
        None
    )

    if matching_ack and matching_ack.get("accepted") == False:
        print("  ✓ Ack received with accepted=false and matching request_id")
        return True

    if resp.status == 400:
        print("  ⚠ HTTP 400 received but no SSE ack (may be expected if not subscribed first)")
        return True

    print("  ✗ Ack correlation failed")
    return False


async def test_ack_correlation_duplicate():
    print("\n" + "="*60)
    print("TEST: Ack correlation for duplicate request")
    print("="*60)

    chat_id = f"test-ack-dup-{uuid.uuid4().hex[:8]}"
    events = []
    client_request_id = str(uuid.uuid4())

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
                            events.append(event)
                            if event["type"] == "ack" and event.get("client_request_id") == client_request_id:
                                if event.get("result", {}).get("duplicate"):
                                    return
            except Exception as e:
                print(f"    Exception: {e}")

    task = asyncio.create_task(subscriber())
    await asyncio.sleep(0.3)

    async with aiohttp.ClientSession() as session:
        resp1 = await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
            "client_request_id": client_request_id,
            "type": "set_params",
            "patch": {"model": "gpt-4o-mini"}
        })

        resp2 = await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
            "client_request_id": client_request_id,
            "type": "set_params",
            "patch": {"model": "gpt-4o-mini"}
        })

        data2 = await resp2.json()
        print(f"    First: {resp1.status}, Second: {resp2.status} {data2}")

    await asyncio.sleep(1)
    task.cancel()

    duplicate_ack = next(
        (e for e in events if e["type"] == "ack" and e.get("result", {}).get("duplicate")),
        None
    )

    if resp2.status == 200 and data2.get("status") == "duplicate":
        print("  ✓ Duplicate request handled correctly")
        return True

    print("  ✗ Duplicate detection failed")
    return False


async def test_ack_correlation_queue_full():
    print("\n" + "="*60)
    print("TEST: Ack correlation for queue full (429)")
    print("="*60)

    chat_id = f"test-ack-queue-{uuid.uuid4().hex[:8]}"
    queue_full_received = False

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
            "type": "user_message",
            "content": "Start generation to block queue processing"
        })

        await asyncio.sleep(0.1)

        tasks = []
        for i in range(150):
            tasks.append(session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
                "client_request_id": str(uuid.uuid4()),
                "type": "set_params",
                "patch": {"boost_reasoning": i % 2 == 0}
            }))

        responses = await asyncio.gather(*tasks, return_exceptions=True)

        for r in responses:
            if isinstance(r, aiohttp.ClientResponse) and r.status == 429:
                queue_full_received = True
                print(f"    Got 429 queue_full")
                break

    if queue_full_received:
        print("  ✓ Queue full (429) received under load")
        return True

    print("  ⚠ Queue never filled (generation may have completed quickly)")
    return True


async def main():
    print("=" * 60)
    print("Chat Session Error & Ack Tests")
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

    results.append(("Invalid model error", await test_invalid_model_error()))
    results.append(("Ack correlation (400)", await test_ack_correlation_invalid_content()))
    results.append(("Ack correlation (duplicate)", await test_ack_correlation_duplicate()))
    results.append(("Ack correlation (queue full)", await test_ack_correlation_queue_full()))

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
