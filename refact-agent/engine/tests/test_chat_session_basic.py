#!/usr/bin/env python3
"""
Basic tests for the stateless trajectory UI (chat session) endpoints.

Run with:
  python tests/test_chat_session_basic.py

Requires:
  - refact-lsp running on port 8001
  - pip install aiohttp
"""

import asyncio
import aiohttp
import json
import uuid
import sys
from typing import List, Dict, Any

LSP_URL = "http://127.0.0.1:8001"


async def test_subscribe_returns_snapshot():
    """Test that subscribing to a chat returns an initial snapshot."""
    print("\n=== Test: Subscribe returns snapshot ===")
    chat_id = f"test-{uuid.uuid4()}"

    async with aiohttp.ClientSession() as session:
        try:
            async with session.get(
                f"{LSP_URL}/v1/chats/subscribe?chat_id={chat_id}",
                timeout=aiohttp.ClientTimeout(total=5)
            ) as resp:
                assert resp.status == 200, f"Expected 200, got {resp.status}"
                assert "text/event-stream" in resp.content_type, \
                    f"Expected SSE, got {resp.content_type}"

                # Read first event
                line = await asyncio.wait_for(
                    resp.content.readline(),
                    timeout=2.0
                )
                assert line.startswith(b"data: "), f"Expected 'data: ', got {line}"

                event = json.loads(line[6:])
                assert event["type"] == "snapshot", \
                    f"Expected snapshot, got {event['type']}"
                assert event["chat_id"] == chat_id
                assert event["runtime"]["state"] == "idle"

                print(f"✓ Received snapshot for chat {chat_id}")
                print(f"  Thread: {event['thread']}")
                print(f"  Runtime: {event['runtime']}")
                return True
        except asyncio.TimeoutError:
            print("✗ Timeout waiting for response")
            return False
        except Exception as e:
            print(f"✗ Error: {e}")
            return False


async def test_send_command_accepted():
    """Test that sending a command returns accepted status."""
    print("\n=== Test: Send command returns accepted ===")
    chat_id = f"test-{uuid.uuid4()}"
    request_id = str(uuid.uuid4())

    async with aiohttp.ClientSession() as session:
        try:
            resp = await session.post(
                f"{LSP_URL}/v1/chats/{chat_id}/commands",
                json={
                    "client_request_id": request_id,
                    "type": "user_message",
                    "content": "Hello, world!",
                },
                timeout=aiohttp.ClientTimeout(total=5)
            )

            assert resp.status == 202, f"Expected 202, got {resp.status}"
            data = await resp.json()
            assert data["status"] == "accepted", \
                f"Expected accepted, got {data}"

            print(f"✓ Command accepted for chat {chat_id}")
            return True
        except Exception as e:
            print(f"✗ Error: {e}")
            return False


async def test_duplicate_command_detected():
    """Test that duplicate commands are detected."""
    print("\n=== Test: Duplicate command detected ===")
    chat_id = f"test-{uuid.uuid4()}"
    request_id = str(uuid.uuid4())

    async with aiohttp.ClientSession() as session:
        try:
            # First request
            resp1 = await session.post(
                f"{LSP_URL}/v1/chats/{chat_id}/commands",
                json={
                    "client_request_id": request_id,
                    "type": "set_params",
                    "patch": {"model": "test-model"},
                }
            )
            assert resp1.status == 202

            # Same request again
            resp2 = await session.post(
                f"{LSP_URL}/v1/chats/{chat_id}/commands",
                json={
                    "client_request_id": request_id,
                    "type": "set_params",
                    "patch": {"model": "test-model"},
                }
            )
            assert resp2.status == 200
            data = await resp2.json()
            assert data["status"] == "duplicate", \
                f"Expected duplicate, got {data}"

            print(f"✓ Duplicate command detected")
            return True
        except Exception as e:
            print(f"✗ Error: {e}")
            return False


async def test_full_message_flow():
    """Test full flow: subscribe, send message, receive events."""
    print("\n=== Test: Full message flow ===")
    chat_id = f"test-{uuid.uuid4()}"
    events: List[Dict[str, Any]] = []

    async def collect_events(max_events: int = 10, timeout: float = 15.0):
        async with aiohttp.ClientSession() as session:
            try:
                async with session.get(
                    f"{LSP_URL}/v1/chats/subscribe?chat_id={chat_id}",
                    timeout=aiohttp.ClientTimeout(total=timeout)
                ) as resp:
                    start_time = asyncio.get_event_loop().time()
                    while len(events) < max_events:
                        if asyncio.get_event_loop().time() - start_time > timeout:
                            break
                        try:
                            line = await asyncio.wait_for(
                                resp.content.readline(),
                                timeout=1.0
                            )
                            if line.startswith(b"data: "):
                                event = json.loads(line[6:])
                                events.append(event)
                                print(f"  Event {len(events)}: {event['type']}")
                                if event["type"] == "stream_finished":
                                    break
                        except asyncio.TimeoutError:
                            continue
            except Exception as e:
                print(f"  Subscription error: {e}")

    # Start subscription in background
    task = asyncio.create_task(collect_events())
    await asyncio.sleep(0.5)  # Let subscription start

    # Send set_params first (to configure model)
    async with aiohttp.ClientSession() as session:
        await session.post(
            f"{LSP_URL}/v1/chats/{chat_id}/commands",
            json={
                "client_request_id": str(uuid.uuid4()),
                "type": "set_params",
                "patch": {"model": "gpt-4o-mini", "mode": "NO_TOOLS"},
            }
        )

        # Send user message
        await session.post(
            f"{LSP_URL}/v1/chats/{chat_id}/commands",
            json={
                "client_request_id": str(uuid.uuid4()),
                "type": "user_message",
                "content": "Say 'Hello' and nothing else.",
            }
        )

    # Wait for events
    try:
        await asyncio.wait_for(task, timeout=20.0)
    except asyncio.TimeoutError:
        print("  (timeout reached)")

    # Analyze events
    event_types = [e["type"] for e in events]
    print(f"\n  Event sequence: {event_types}")

    # Check expected events
    checks = [
        ("snapshot", "snapshot" in event_types),
        ("message_added (user)", event_types.count("message_added") >= 1),
        ("stream_started", "stream_started" in event_types),
    ]

    all_passed = True
    for name, passed in checks:
        status = "✓" if passed else "✗"
        print(f"  {status} {name}")
        all_passed = all_passed and passed

    # stream_delta and stream_finished may not appear if model not configured
    if "stream_delta" in event_types:
        print(f"  ✓ stream_delta received")
    else:
        print(f"  ⚠ No stream_delta (model may not be configured)")

    if "stream_finished" in event_types:
        print(f"  ✓ stream_finished received")

    return all_passed


async def test_abort_command():
    """Test aborting a generation."""
    print("\n=== Test: Abort command ===")
    chat_id = f"test-{uuid.uuid4()}"

    async with aiohttp.ClientSession() as session:
        # Send abort (even without active generation)
        resp = await session.post(
            f"{LSP_URL}/v1/chats/{chat_id}/commands",
            json={
                "client_request_id": str(uuid.uuid4()),
                "type": "abort",
            }
        )
        # Abort is handled immediately, returns 200 with aborted status
        assert resp.status == 200
        data = await resp.json()
        assert data.get("status") == "aborted"
        print(f"✓ Abort command handled immediately")
        return True


async def main():
    print("=" * 60)
    print("Chat Session Endpoint Tests")
    print("=" * 60)
    print(f"Testing against: {LSP_URL}")

    # Check if server is running
    try:
        async with aiohttp.ClientSession() as session:
            async with session.get(f"{LSP_URL}/v1/ping", timeout=aiohttp.ClientTimeout(total=2)) as resp:
                if resp.status != 200:
                    print(f"\n✗ Server not responding correctly at {LSP_URL}")
                    sys.exit(1)
    except Exception as e:
        print(f"\n✗ Cannot connect to server at {LSP_URL}: {e}")
        print("  Make sure refact-lsp is running with: cargo run")
        sys.exit(1)

    print("✓ Server is running\n")

    results = []

    # Run tests
    results.append(("Subscribe returns snapshot", await test_subscribe_returns_snapshot()))
    results.append(("Send command accepted", await test_send_command_accepted()))
    results.append(("Duplicate command detected", await test_duplicate_command_detected()))
    results.append(("Abort command", await test_abort_command()))
    results.append(("Full message flow", await test_full_message_flow()))

    # Summary
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
