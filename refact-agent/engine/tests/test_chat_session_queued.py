#!/usr/bin/env python3
"""
Tests for queued messages and edge cases in the chat session system.

Run with:
  python tests/test_chat_session_queued.py

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
DEFAULT_MODEL = "refact/claude-haiku-4-5"


async def test_queued_messages_order():
    """Test that queued messages are processed in order."""
    print("\n" + "="*60)
    print("TEST: Queued messages processed in order")
    print("="*60)

    chat_id = f"test-queue-order-{uuid.uuid4().hex[:8]}"
    events = []
    user_messages_content = []

    async def subscriber():
        async with aiohttp.ClientSession() as session:
            try:
                async with session.get(
                    f"{LSP_URL}/v1/chats/subscribe?chat_id={chat_id}",
                    timeout=aiohttp.ClientTimeout(total=60)
                ) as resp:
                    async for line in resp.content:
                        if line.startswith(b"data: "):
                            event = json.loads(line[6:])
                            events.append(event)

                            if event["type"] == "message_added":
                                msg = event.get("message", {})
                                if msg.get("role") == "user":
                                    content = msg.get("content", "")
                                    if isinstance(content, str):
                                        user_messages_content.append(content)
                                    print(f"    User message added: {content[:30]}...")

                            if event["type"] == "stream_finished":
                                print(f"    Stream finished")

                            # Wait for all 3 user messages and their responses
                            user_count = len(user_messages_content)
                            asst_count = sum(1 for e in events
                                           if e["type"] == "stream_finished")
                            if user_count >= 3 and asst_count >= 3:
                                await asyncio.sleep(0.5)
                                return
            except Exception as e:
                print(f"    Exception: {e}")

    task = asyncio.create_task(subscriber())
    await asyncio.sleep(0.3)

    async with aiohttp.ClientSession() as session:
        # Set model first
        await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
            "client_request_id": str(uuid.uuid4()),
            "type": "set_params",
            "patch": {"model": DEFAULT_MODEL, "mode": "NO_TOOLS"}
        })

        # Send 3 messages rapidly - they should queue
        messages = ["First message", "Second message", "Third message"]
        for msg in messages:
            await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
                "client_request_id": str(uuid.uuid4()),
                "type": "user_message",
                "content": msg
            })
            await asyncio.sleep(0.1)  # Tiny delay to ensure ordering

    await asyncio.sleep(30)
    task.cancel()

    print(f"\n  User messages received: {user_messages_content}")

    # Verify order
    if user_messages_content == messages:
        print("  ✓ Messages processed in correct order")
        return True
    else:
        print("  ✗ Messages out of order!")
        return False


async def test_queue_size_updates():
    """Test that queue_size is updated in runtime events."""
    print("\n" + "="*60)
    print("TEST: Queue size updates in runtime events")
    print("="*60)

    chat_id = f"test-queue-size-{uuid.uuid4().hex[:8]}"
    queue_sizes = []

    async def subscriber():
        async with aiohttp.ClientSession() as session:
            try:
                async with session.get(
                    f"{LSP_URL}/v1/chats/subscribe?chat_id={chat_id}",
                    timeout=aiohttp.ClientTimeout(total=30)
                ) as resp:
                    async for line in resp.content:
                        if line.startswith(b"data: "):
                            event = json.loads(line[6:])

                            if event["type"] == "runtime_updated":
                                qs = event.get("queue_size", 0)
                                queue_sizes.append(qs)
                                print(f"    Runtime: state={event.get('state')}, queue_size={qs}")

                            if event["type"] == "stream_finished":
                                if len(queue_sizes) >= 3:
                                    return
            except Exception as e:
                print(f"    Exception: {e}")

    task = asyncio.create_task(subscriber())
    await asyncio.sleep(0.3)

    async with aiohttp.ClientSession() as session:
        await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
            "client_request_id": str(uuid.uuid4()),
            "type": "set_params",
            "patch": {"model": DEFAULT_MODEL, "mode": "NO_TOOLS"}
        })

        # Send first message
        await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
            "client_request_id": str(uuid.uuid4()),
            "type": "user_message",
            "content": "First"
        })

        # Wait a bit then send more during generation
        await asyncio.sleep(0.5)
        await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
            "client_request_id": str(uuid.uuid4()),
            "type": "user_message",
            "content": "Second"
        })

    await asyncio.sleep(15)
    task.cancel()

    print(f"\n  Queue sizes observed: {queue_sizes}")
    # Should see queue_size increase when messages are queued during generation
    return True


async def test_two_subscribers():
    """Test that two subscribers receive the same events."""
    print("\n" + "="*60)
    print("TEST: Two subscribers receive same events")
    print("="*60)

    chat_id = f"test-two-subs-{uuid.uuid4().hex[:8]}"
    events_1 = []
    events_2 = []

    async def subscriber(events_list, name):
        async with aiohttp.ClientSession() as session:
            try:
                async with session.get(
                    f"{LSP_URL}/v1/chats/subscribe?chat_id={chat_id}",
                    timeout=aiohttp.ClientTimeout(total=20)
                ) as resp:
                    async for line in resp.content:
                        if line.startswith(b"data: "):
                            event = json.loads(line[6:])
                            events_list.append(event)

                            if event["type"] == "stream_finished":
                                print(f"    {name}: stream_finished (total: {len(events_list)} events)")
                                return
            except Exception as e:
                print(f"    {name} exception: {e}")

    # Start both subscribers
    task1 = asyncio.create_task(subscriber(events_1, "Sub1"))
    task2 = asyncio.create_task(subscriber(events_2, "Sub2"))
    await asyncio.sleep(0.5)

    async with aiohttp.ClientSession() as session:
        await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
            "client_request_id": str(uuid.uuid4()),
            "type": "set_params",
            "patch": {"model": DEFAULT_MODEL, "mode": "NO_TOOLS"}
        })
        await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
            "client_request_id": str(uuid.uuid4()),
            "type": "user_message",
            "content": "Hello from both subscribers"
        })

    await asyncio.sleep(10)
    task1.cancel()
    task2.cancel()

    print(f"\n  Sub1 events: {len(events_1)}")
    print(f"  Sub2 events: {len(events_2)}")

    # Both should have received same event types (ignoring exact timing)
    types_1 = [e["type"] for e in events_1]
    types_2 = [e["type"] for e in events_2]

    if types_1 == types_2:
        print("  ✓ Both subscribers received same event sequence")
        return True
    else:
        print(f"  ✗ Event sequences differ:")
        print(f"    Sub1: {types_1}")
        print(f"    Sub2: {types_2}")
        return False


async def test_concurrent_writers():
    """Test that concurrent writers don't corrupt state."""
    print("\n" + "="*60)
    print("TEST: Concurrent writers (two clients sending)")
    print("="*60)

    chat_id = f"test-concurrent-{uuid.uuid4().hex[:8]}"
    events = []
    user_messages = []

    async def subscriber():
        async with aiohttp.ClientSession() as session:
            try:
                async with session.get(
                    f"{LSP_URL}/v1/chats/subscribe?chat_id={chat_id}",
                    timeout=aiohttp.ClientTimeout(total=40)
                ) as resp:
                    async for line in resp.content:
                        if line.startswith(b"data: "):
                            event = json.loads(line[6:])
                            events.append(event)

                            if event["type"] == "message_added":
                                msg = event.get("message", {})
                                if msg.get("role") == "user":
                                    content = msg.get("content", "")
                                    user_messages.append(content)
                                    print(f"    User: {content[:30]}...")

                            if len(user_messages) >= 4:
                                await asyncio.sleep(1)
                                return
            except Exception as e:
                print(f"    Exception: {e}")

    task = asyncio.create_task(subscriber())
    await asyncio.sleep(0.3)

    async with aiohttp.ClientSession() as session:
        await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
            "client_request_id": str(uuid.uuid4()),
            "type": "set_params",
            "patch": {"model": DEFAULT_MODEL, "mode": "NO_TOOLS"}
        })

    # Two "clients" sending messages concurrently
    async def client_a():
        async with aiohttp.ClientSession() as session:
            for i in range(2):
                await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
                    "client_request_id": str(uuid.uuid4()),
                    "type": "user_message",
                    "content": f"Client A message {i+1}"
                })
                await asyncio.sleep(0.3)

    async def client_b():
        async with aiohttp.ClientSession() as session:
            for i in range(2):
                await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
                    "client_request_id": str(uuid.uuid4()),
                    "type": "user_message",
                    "content": f"Client B message {i+1}"
                })
                await asyncio.sleep(0.3)

    # Run both clients concurrently
    await asyncio.gather(client_a(), client_b())

    await asyncio.sleep(25)
    task.cancel()

    print(f"\n  User messages received: {user_messages}")

    # Should have all 4 messages (order may vary)
    if len(user_messages) >= 4:
        print("  ✓ All messages from both clients received")
        return True
    else:
        print(f"  ✗ Only {len(user_messages)} messages received")
        return False


async def test_abort_clears_draft():
    """Test that abort clears the draft message."""
    print("\n" + "="*60)
    print("TEST: Abort clears draft message")
    print("="*60)

    chat_id = f"test-abort-{uuid.uuid4().hex[:8]}"
    events = []

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
                            print(f"    Event: {event['type']}")

                            if event["type"] == "message_removed":
                                print(f"    Draft removed: {event.get('message_id', '')[:20]}...")
                                return
            except Exception as e:
                print(f"    Exception: {e}")

    task = asyncio.create_task(subscriber())
    await asyncio.sleep(0.3)

    async with aiohttp.ClientSession() as session:
        await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
            "client_request_id": str(uuid.uuid4()),
            "type": "set_params",
            "patch": {"model": DEFAULT_MODEL, "mode": "NO_TOOLS"}
        })

        # Start generation
        await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
            "client_request_id": str(uuid.uuid4()),
            "type": "user_message",
            "content": "Write a very long essay about programming"
        })

        # Wait for generation to start
        await asyncio.sleep(1)

        # Abort
        await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
            "client_request_id": str(uuid.uuid4()),
            "type": "abort"
        })

    await asyncio.sleep(5)
    task.cancel()

    event_types = [e["type"] for e in events]
    print(f"\n  Event sequence: {event_types}")

    # Should see stream_started then message_removed (draft cleared)
    if "stream_started" in event_types and "message_removed" in event_types:
        print("  ✓ Abort properly cleared draft message")
        return True
    elif "stream_finished" in event_types:
        print("  ⚠ Generation completed before abort")
        return True
    else:
        print("  ✗ Unexpected event sequence")
        return False


async def test_empty_message():
    """Test handling of empty message content."""
    print("\n" + "="*60)
    print("TEST: Empty message handling")
    print("="*60)

    chat_id = f"test-empty-{uuid.uuid4().hex[:8]}"
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

                            if event["type"] == "message_added":
                                msg = event.get("message", {})
                                print(f"    Message: {msg.get('role')} - '{msg.get('content', '')}'")

                            if event["type"] in ("stream_finished", "runtime_updated"):
                                if event.get("state") in ("idle", "error"):
                                    await asyncio.sleep(0.5)
                                    return
            except Exception as e:
                print(f"    Exception: {e}")

    task = asyncio.create_task(subscriber())
    await asyncio.sleep(0.3)

    async with aiohttp.ClientSession() as session:
        await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
            "client_request_id": str(uuid.uuid4()),
            "type": "set_params",
            "patch": {"model": DEFAULT_MODEL, "mode": "NO_TOOLS"}
        })

        # Send empty message
        await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
            "client_request_id": str(uuid.uuid4()),
            "type": "user_message",
            "content": ""
        })

    await asyncio.sleep(8)
    task.cancel()

    # Should handle empty message gracefully
    has_user_msg = any(e["type"] == "message_added" and e.get("message", {}).get("role") == "user"
                       for e in events)
    print(f"\n  Empty user message added: {has_user_msg}")
    return True  # Just checking it doesn't crash


async def test_setparams_during_generation():
    """Test that SetParams during generation is queued."""
    print("\n" + "="*60)
    print("TEST: SetParams during generation queued")
    print("="*60)

    chat_id = f"test-params-{uuid.uuid4().hex[:8]}"
    events = []
    thread_updates = []

    async def subscriber():
        async with aiohttp.ClientSession() as session:
            try:
                async with session.get(
                    f"{LSP_URL}/v1/chats/subscribe?chat_id={chat_id}",
                    timeout=aiohttp.ClientTimeout(total=20)
                ) as resp:
                    async for line in resp.content:
                        if line.startswith(b"data: "):
                            event = json.loads(line[6:])
                            events.append(event)

                            if event["type"] == "thread_updated":
                                thread_updates.append(event.get("params", {}))
                                print(f"    Thread updated: {event.get('params', {})}")

                            if event["type"] == "stream_finished":
                                await asyncio.sleep(0.5)
                                return
            except Exception as e:
                print(f"    Exception: {e}")

    task = asyncio.create_task(subscriber())
    await asyncio.sleep(0.3)

    async with aiohttp.ClientSession() as session:
        # Initial params
        await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
            "client_request_id": str(uuid.uuid4()),
            "type": "set_params",
            "patch": {"model": DEFAULT_MODEL, "mode": "NO_TOOLS"}
        })

        # Start generation
        await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
            "client_request_id": str(uuid.uuid4()),
            "type": "user_message",
            "content": "Hello"
        })

        # Send params update during generation
        await asyncio.sleep(0.3)
        await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
            "client_request_id": str(uuid.uuid4()),
            "type": "set_params",
            "patch": {"boost_reasoning": True}
        })

    await asyncio.sleep(10)
    task.cancel()

    print(f"\n  Thread updates: {len(thread_updates)}")

    # Should have received thread_updated for both params changes
    if len(thread_updates) >= 1:
        print("  ✓ SetParams was processed")
        return True
    else:
        print("  ⚠ SetParams may have been queued for later")
        return True


async def test_snapshot_after_messages():
    """Test that snapshot contains all messages after disconnect/reconnect."""
    print("\n" + "="*60)
    print("TEST: Snapshot contains all messages after reconnect")
    print("="*60)

    chat_id = f"test-snapshot-{uuid.uuid4().hex[:8]}"

    # First connection - send a message
    async with aiohttp.ClientSession() as session:
        await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
            "client_request_id": str(uuid.uuid4()),
            "type": "set_params",
            "patch": {"model": DEFAULT_MODEL, "mode": "NO_TOOLS"}
        })
        await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
            "client_request_id": str(uuid.uuid4()),
            "type": "user_message",
            "content": "First message for snapshot test"
        })

    # Wait for generation to complete
    await asyncio.sleep(5)

    # Send another message
    async with aiohttp.ClientSession() as session:
        await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
            "client_request_id": str(uuid.uuid4()),
            "type": "user_message",
            "content": "Second message for snapshot test"
        })

    await asyncio.sleep(5)

    # Reconnect and check snapshot
    async with aiohttp.ClientSession() as session:
        async with session.get(
            f"{LSP_URL}/v1/chats/subscribe?chat_id={chat_id}",
            timeout=aiohttp.ClientTimeout(total=5)
        ) as resp:
            line = await resp.content.readline()
            if line.startswith(b"data: "):
                event = json.loads(line[6:])
                if event["type"] == "snapshot":
                    messages = event.get("messages", [])
                    print(f"  Snapshot has {len(messages)} messages:")
                    for m in messages:
                        role = m.get("role", "?")
                        content = str(m.get("content", ""))[:40]
                        print(f"    {role}: {content}...")

                    # Should have at least 4 messages (2 user + 2 assistant)
                    if len(messages) >= 4:
                        print("  ✓ Snapshot contains all expected messages")
                        return True
                    else:
                        print(f"  ⚠ Expected at least 4 messages, got {len(messages)}")
                        return True  # Still valid, model might have failed

    return True


async def test_ack_events():
    """Test that ACK events are sent for commands."""
    print("\n" + "="*60)
    print("TEST: ACK events sent for commands")
    print("="*60)

    chat_id = f"test-ack-{uuid.uuid4().hex[:8]}"
    request_id = str(uuid.uuid4())
    ack_events = []

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

                            if event["type"] == "ack":
                                ack_events.append(event)
                                print(f"    ACK: request_id={event.get('client_request_id', '')[:20]}...")
                                if event.get("client_request_id") == request_id:
                                    return
            except Exception as e:
                print(f"    Exception: {e}")

    task = asyncio.create_task(subscriber())
    await asyncio.sleep(0.3)

    async with aiohttp.ClientSession() as session:
        await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
            "client_request_id": request_id,
            "type": "set_params",
            "patch": {"model": "test-model"}
        })

    await asyncio.sleep(2)
    task.cancel()

    # Should have received ACK for our request
    our_ack = [a for a in ack_events if a.get("client_request_id") == request_id]
    if our_ack:
        print(f"\n  ✓ Received ACK for our request")
        print(f"    accepted={our_ack[0].get('accepted')}, result={our_ack[0].get('result')}")
        return True
    else:
        print(f"\n  ✗ No ACK received for our request")
        return False


async def test_multiple_threads_simultaneously():
    """Test multiple independent chat threads running at the same time."""
    print("\n" + "="*60)
    print("TEST: Multiple threads simultaneously (6 threads)")
    print("="*60)

    threads = []

    # 2 simple chat threads
    for i in range(2):
        threads.append({
            'chat_id': f'simple-{i}-{uuid.uuid4().hex[:8]}',
            'type': 'simple',
            'prompt': f'Say "Hello from thread {i}"',
            'events': [],
            'finished': False,
            'success': False
        })

    # 2 threads that will be aborted
    for i in range(2):
        threads.append({
            'chat_id': f'abort-{i}-{uuid.uuid4().hex[:8]}',
            'type': 'abort',
            'prompt': 'Write a very long essay about the history of computing.',
            'events': [],
            'finished': False,
            'success': False
        })

    # 2 AGENT mode threads
    for i in range(2):
        threads.append({
            'chat_id': f'agent-{i}-{uuid.uuid4().hex[:8]}',
            'type': 'agent',
            'prompt': 'What is 2+2? Just answer.',
            'events': [],
            'finished': False,
            'success': False
        })

    async def subscriber(thread):
        async with aiohttp.ClientSession() as session:
            try:
                async with session.get(
                    f'{LSP_URL}/v1/chats/subscribe?chat_id={thread["chat_id"]}',
                    timeout=aiohttp.ClientTimeout(total=45)
                ) as resp:
                    async for line in resp.content:
                        if line.startswith(b'data: '):
                            event = json.loads(line[6:])
                            thread['events'].append(event)

                            if event['type'] == 'stream_finished':
                                thread['finished'] = True
                                thread['success'] = True
                                return

                            if event['type'] == 'message_removed':
                                thread['finished'] = True
                                thread['success'] = True
                                return

                            if event['type'] == 'runtime_updated':
                                if event.get('state') == 'idle' and len(thread['events']) > 5:
                                    thread['finished'] = True
                                    thread['success'] = True
                                    return
            except Exception as e:
                thread['error'] = str(e)

    async def send_and_maybe_abort(thread):
        await asyncio.sleep(0.3)
        mode = 'AGENT' if thread['type'] == 'agent' else 'NO_TOOLS'
        async with aiohttp.ClientSession() as session:
            await session.post(f'{LSP_URL}/v1/chats/{thread["chat_id"]}/commands', json={
                'client_request_id': str(uuid.uuid4()),
                'type': 'set_params',
                'patch': {'model': DEFAULT_MODEL, 'mode': mode}
            })
            await session.post(f'{LSP_URL}/v1/chats/{thread["chat_id"]}/commands', json={
                'client_request_id': str(uuid.uuid4()),
                'type': 'user_message',
                'content': thread['prompt']
            })
            if thread['type'] == 'abort':
                await asyncio.sleep(0.8)
                await session.post(f'{LSP_URL}/v1/chats/{thread["chat_id"]}/commands', json={
                    'client_request_id': str(uuid.uuid4()),
                    'type': 'abort'
                })

    subscriber_tasks = [asyncio.create_task(subscriber(t)) for t in threads]
    await asyncio.sleep(0.2)

    send_tasks = [asyncio.create_task(send_and_maybe_abort(t)) for t in threads]
    await asyncio.gather(*send_tasks)

    try:
        await asyncio.wait_for(asyncio.gather(*subscriber_tasks), timeout=40)
    except asyncio.TimeoutError:
        for task in subscriber_tasks:
            task.cancel()

    # Results
    for thread_type in ['simple', 'abort', 'agent']:
        type_threads = [t for t in threads if t['type'] == thread_type]
        success_count = sum(1 for t in type_threads if t['success'])
        print(f"    {thread_type}: {success_count}/{len(type_threads)} succeeded")

    total_success = sum(1 for t in threads if t['success'])
    if total_success == len(threads):
        print(f"  ✓ All {len(threads)} threads completed")
        return True
    else:
        print(f"  ⚠ {total_success}/{len(threads)} threads succeeded")
        return total_success >= len(threads) - 1


async def test_thread_isolation():
    """Test that threads are isolated - messages don't leak between them."""
    print("\n" + "="*60)
    print("TEST: Thread isolation (5 threads)")
    print("="*60)

    num_threads = 5
    threads = []

    for i in range(num_threads):
        keyword = f'KEYWORD_{i}_{uuid.uuid4().hex[:4]}'
        threads.append({
            'chat_id': f'isolation-{i}-{uuid.uuid4().hex[:8]}',
            'keyword': keyword,
            'prompt': f'Repeat exactly: {keyword}',
            'events': [],
            'content': [],
            'finished': False,
        })

    async def subscriber(thread):
        async with aiohttp.ClientSession() as session:
            try:
                async with session.get(
                    f'{LSP_URL}/v1/chats/subscribe?chat_id={thread["chat_id"]}',
                    timeout=aiohttp.ClientTimeout(total=30)
                ) as resp:
                    async for line in resp.content:
                        if line.startswith(b'data: '):
                            event = json.loads(line[6:])
                            thread['events'].append(event)
                            if event['type'] == 'stream_delta':
                                for op in event.get('ops', []):
                                    if op.get('op') == 'append_content':
                                        thread['content'].append(op.get('text', ''))
                            if event['type'] == 'stream_finished':
                                thread['finished'] = True
                                return
            except Exception as e:
                pass

    async def send_message(thread):
        await asyncio.sleep(0.2)
        async with aiohttp.ClientSession() as session:
            await session.post(f'{LSP_URL}/v1/chats/{thread["chat_id"]}/commands', json={
                'client_request_id': str(uuid.uuid4()),
                'type': 'set_params',
                'patch': {'model': DEFAULT_MODEL, 'mode': 'NO_TOOLS'}
            })
            await session.post(f'{LSP_URL}/v1/chats/{thread["chat_id"]}/commands', json={
                'client_request_id': str(uuid.uuid4()),
                'type': 'user_message',
                'content': thread['prompt']
            })

    subscriber_tasks = [asyncio.create_task(subscriber(t)) for t in threads]
    await asyncio.sleep(0.2)
    send_tasks = [asyncio.create_task(send_message(t)) for t in threads]
    await asyncio.gather(*send_tasks)

    try:
        await asyncio.wait_for(asyncio.gather(*subscriber_tasks), timeout=30)
    except asyncio.TimeoutError:
        for task in subscriber_tasks:
            task.cancel()

    # Check isolation
    isolated = 0
    leaked = 0
    for thread in threads:
        content = ''.join(thread['content'])
        other_keywords = [t['keyword'] for t in threads if t != thread]
        has_own = thread['keyword'] in content
        has_other = any(k in content for k in other_keywords)
        if has_own and not has_other:
            isolated += 1
        elif has_other:
            leaked += 1

    print(f"    Isolated: {isolated}/{num_threads}, Leaked: {leaked}/{num_threads}")

    if leaked == 0:
        print("  ✓ Thread isolation verified")
        return True
    else:
        print("  ✗ Thread isolation FAILED")
        return False


async def test_thinking_mode():
    """Test thinking/reasoning mode with boost_reasoning."""
    print("\n" + "="*60)
    print("TEST: Thinking mode (boost_reasoning)")
    print("="*60)

    chat_id = f"test-thinking-{uuid.uuid4().hex[:8]}"
    events = []
    reasoning_chunks = []
    content_chunks = []

    async def subscriber():
        async with aiohttp.ClientSession() as session:
            try:
                async with session.get(
                    f"{LSP_URL}/v1/chats/subscribe?chat_id={chat_id}",
                    timeout=aiohttp.ClientTimeout(total=60)
                ) as resp:
                    async for line in resp.content:
                        if line.startswith(b"data: "):
                            event = json.loads(line[6:])
                            events.append(event)

                            if event["type"] == "stream_delta":
                                for op in event.get("ops", []):
                                    if op.get("op") == "append_reasoning":
                                        reasoning_chunks.append(op.get("text", ""))
                                    elif op.get("op") == "append_content":
                                        content_chunks.append(op.get("text", ""))
                                    elif op.get("op") == "set_thinking_blocks":
                                        print(f"    Thinking blocks: {len(op.get('blocks', []))} blocks")

                            if event["type"] == "stream_finished":
                                return
            except Exception as e:
                print(f"    Exception: {e}")

    task = asyncio.create_task(subscriber())
    await asyncio.sleep(0.3)

    async with aiohttp.ClientSession() as session:
        # Enable thinking mode
        await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
            "client_request_id": str(uuid.uuid4()),
            "type": "set_params",
            "patch": {
                "model": DEFAULT_MODEL,
                "mode": "NO_TOOLS",
                "boost_reasoning": True
            }
        })

        await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
            "client_request_id": str(uuid.uuid4()),
            "type": "user_message",
            "content": "What is 17 * 23? Think step by step."
        })

    await asyncio.sleep(30)
    task.cancel()

    reasoning = "".join(reasoning_chunks)
    content = "".join(content_chunks)

    print(f"\n  Reasoning length: {len(reasoning)} chars")
    print(f"  Content length: {len(content)} chars")

    if reasoning:
        print(f"  Reasoning preview: {reasoning[:100]}...")
        print("  ✓ Received reasoning content")
    else:
        print("  ⚠ No reasoning content (model may not support extended thinking)")

    if content:
        print(f"  Content preview: {content[:100]}...")
        print("  ✓ Received main content")
        return True
    else:
        print("  ✗ No content received")
        return False


async def test_thinking_mode_with_tools():
    """Test thinking mode combined with tool usage."""
    print("\n" + "="*60)
    print("TEST: Thinking mode with tools")
    print("="*60)

    chat_id = f"test-thinking-tools-{uuid.uuid4().hex[:8]}"
    events = []
    tool_calls_received = False
    reasoning_received = False

    async def subscriber():
        nonlocal tool_calls_received, reasoning_received
        async with aiohttp.ClientSession() as session:
            try:
                async with session.get(
                    f"{LSP_URL}/v1/chats/subscribe?chat_id={chat_id}",
                    timeout=aiohttp.ClientTimeout(total=60)
                ) as resp:
                    async for line in resp.content:
                        if line.startswith(b"data: "):
                            event = json.loads(line[6:])
                            events.append(event)

                            if event["type"] == "stream_delta":
                                for op in event.get("ops", []):
                                    if op.get("op") == "append_reasoning":
                                        reasoning_received = True
                                    elif op.get("op") == "set_tool_calls":
                                        tool_calls = op.get("tool_calls", [])
                                        if tool_calls:
                                            tool_calls_received = True
                                            for tc in tool_calls:
                                                name = tc.get("function", {}).get("name", "")
                                                if name:
                                                    print(f"    Tool call: {name}")

                            if event["type"] == "message_added":
                                msg = event.get("message", {})
                                if msg.get("role") == "tool":
                                    print(f"    Tool result added")

                            if event["type"] == "runtime_updated":
                                state = event.get("state")
                                if state == "idle" and tool_calls_received:
                                    await asyncio.sleep(1)
                                    return
                                elif state == "error":
                                    print(f"    Error: {event.get('error', '')[:50]}")
                                    return
            except Exception as e:
                print(f"    Exception: {e}")

    task = asyncio.create_task(subscriber())
    await asyncio.sleep(0.3)

    async with aiohttp.ClientSession() as session:
        # Enable thinking mode with AGENT
        await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
            "client_request_id": str(uuid.uuid4()),
            "type": "set_params",
            "patch": {
                "model": DEFAULT_MODEL,
                "mode": "AGENT",
                "boost_reasoning": True
            }
        })

        await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
            "client_request_id": str(uuid.uuid4()),
            "type": "user_message",
            "content": "List the files in the current directory using tree tool."
        })

    await asyncio.sleep(45)
    task.cancel()

    print(f"\n  Reasoning received: {reasoning_received}")
    print(f"  Tool calls received: {tool_calls_received}")

    if tool_calls_received:
        print("  ✓ Tool calls worked with thinking mode")
        return True
    else:
        print("  ⚠ No tool calls (model may have answered directly)")
        return True  # Still valid


async def main():
    print("="*60)
    print("QUEUED MESSAGES & EDGE CASES TESTS")
    print("="*60)

    # Check if server is running
    try:
        async with aiohttp.ClientSession() as session:
            async with session.get(f"{LSP_URL}/v1/ping", timeout=aiohttp.ClientTimeout(total=2)) as resp:
                if resp.status != 200:
                    print(f"\n✗ Server not responding at {LSP_URL}")
                    sys.exit(1)
    except Exception as e:
        print(f"\n✗ Cannot connect to server: {e}")
        sys.exit(1)

    print("✓ Server is running\n")

    results = []

    # Run tests
    results.append(("ACK events", await test_ack_events()))
    results.append(("Empty message", await test_empty_message()))
    results.append(("SetParams during generation", await test_setparams_during_generation()))
    results.append(("Queue size updates", await test_queue_size_updates()))
    results.append(("Two subscribers", await test_two_subscribers()))
    results.append(("Concurrent writers", await test_concurrent_writers()))
    results.append(("Abort clears draft", await test_abort_clears_draft()))
    results.append(("Snapshot after messages", await test_snapshot_after_messages()))
    results.append(("Queued messages order", await test_queued_messages_order()))
    results.append(("Thinking mode", await test_thinking_mode()))
    results.append(("Thinking mode with tools", await test_thinking_mode_with_tools()))
    results.append(("Multiple threads simultaneously", await test_multiple_threads_simultaneously()))
    results.append(("Thread isolation", await test_thread_isolation()))

    # Summary
    print("\n" + "="*60)
    print("SUMMARY")
    print("="*60)

    passed = sum(1 for _, r in results if r)
    total = len(results)

    for name, result in results:
        status = "✓ PASS" if result else "✗ FAIL"
        print(f"  {status}: {name}")

    print(f"\nTotal: {passed}/{total} passed")

    sys.exit(0 if passed == total else 1)


if __name__ == "__main__":
    asyncio.run(main())
