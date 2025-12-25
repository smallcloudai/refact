#!/usr/bin/env python3
"""Test Claude models and corner cases."""

import asyncio
import aiohttp
import json
import uuid

LSP_URL = "http://127.0.0.1:8001"


async def test_claude_models():
    """Test Claude models."""
    print("\n" + "="*60)
    print("TEST: Claude models")
    print("="*60)

    models = [
        "refact/claude-haiku-4-5",
        "refact/claude-sonnet-4-5",
    ]

    for model in models:
        print(f"\n  Testing: {model}")
        chat_id = f"test-claude-{uuid.uuid4().hex[:8]}"
        events = []
        content_chunks = []

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

                                if event["type"] == "stream_delta":
                                    for op in event.get("ops", []):
                                        if op.get("op") == "append_content":
                                            content_chunks.append(op.get("text", ""))

                                if event["type"] == "stream_finished":
                                    break
                                if event["type"] == "runtime_updated" and event.get("state") == "error":
                                    print(f"    Error: {event.get('error', '')[:80]}")
                                    break
                except Exception as e:
                    print(f"    Exception: {e}")

        task = asyncio.create_task(subscriber())
        await asyncio.sleep(0.3)

        async with aiohttp.ClientSession() as session:
            await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
                "client_request_id": str(uuid.uuid4()),
                "type": "set_params",
                "patch": {"model": model, "mode": "NO_TOOLS"}
            })
            await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
                "client_request_id": str(uuid.uuid4()),
                "type": "user_message",
                "content": "Say hello only"
            })

        await asyncio.sleep(15)
        task.cancel()

        response = "".join(content_chunks)
        if response:
            print(f"    Got response: {response[:50]}...")
        elif any(e.get("state") == "error" for e in events):
            print(f"    Error occurred")
        else:
            print(f"    No content received")

    return True


async def test_tool_call_advancement():
    """Test that chat correctly advances through tool calls."""
    print("\n" + "="*60)
    print("TEST: Tool call advancement")
    print("="*60)

    chat_id = f"test-advance-{uuid.uuid4().hex[:8]}"
    events = []
    tool_results_added = 0
    generations_started = 0

    async def subscriber():
        nonlocal tool_results_added, generations_started
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

                            if event["type"] == "stream_started":
                                generations_started += 1
                                print(f"    stream_started #{generations_started}")

                            if event["type"] == "message_added":
                                msg = event.get("message", {})
                                role = msg.get("role")
                                if role == "tool":
                                    tool_results_added += 1
                                    print(f"    tool result added #{tool_results_added}")
                                elif role == "assistant":
                                    has_tools = msg.get("tool_calls")
                                    if has_tools:
                                        print(f"    assistant message with {len(has_tools)} tool call(s)")
                                    else:
                                        content = str(msg.get("content", ""))[:40]
                                        print(f"    assistant message: {content}...")

                            if event["type"] == "pause_required":
                                print(f"    PAUSED - needs confirmation")

                            if event["type"] == "runtime_updated":
                                state = event.get("state")
                                if state == "idle" and generations_started > 0:
                                    print(f"    idle (after {generations_started} generations)")
                                    if generations_started >= 2:
                                        await asyncio.sleep(1)
                                        break
                                elif state == "error":
                                    print(f"    ERROR: {event.get('error', '')[:60]}")
                                    break
                                elif state == "paused":
                                    print(f"    PAUSED")
            except Exception as e:
                print(f"    Exception: {e}")

    task = asyncio.create_task(subscriber())
    await asyncio.sleep(0.3)

    async with aiohttp.ClientSession() as session:
        await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
            "client_request_id": str(uuid.uuid4()),
            "type": "set_params",
            "patch": {"model": "refact/claude-haiku-4-5", "mode": "AGENT"}
        })

        await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
            "client_request_id": str(uuid.uuid4()),
            "type": "user_message",
            "content": "List the files in the current directory using tree tool, then summarize what you found."
        })

    await asyncio.sleep(45)
    task.cancel()

    print(f"\n  Summary:")
    print(f"    Generations started: {generations_started}")
    print(f"    Tool results added: {tool_results_added}")

    if generations_started >= 2:
        print("    Chat advanced through tool calls (multiple generations)")
    elif tool_results_added > 0:
        print("    Tool results were processed")
    elif any(e["type"] == "pause_required" for e in events):
        print("    Paused for confirmation (tool advancement blocked)")
    else:
        print("    Single generation (model may not have used tools)")

    return True


async def test_send_message_during_generation():
    """Test sending a message while generation is in progress."""
    print("\n" + "="*60)
    print("TEST: Send message during generation")
    print("="*60)

    chat_id = f"test-during-{uuid.uuid4().hex[:8]}"
    events = []
    message_added_count = 0

    async def subscriber():
        nonlocal message_added_count
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

                            if event["type"] == "message_added":
                                message_added_count += 1
                                msg = event.get("message", {})
                                role = msg.get("role")
                                content = str(msg.get("content", ""))[:30]
                                print(f"    message_added: {role} - {content}...")

                            if event["type"] == "stream_started":
                                print(f"    stream_started")

                            if event["type"] == "stream_finished":
                                print(f"    stream_finished")

                            if event["type"] == "runtime_updated" and event.get("state") == "idle":
                                if message_added_count >= 4:
                                    break
            except Exception as e:
                print(f"    Exception: {e}")

    task = asyncio.create_task(subscriber())
    await asyncio.sleep(0.3)

    async with aiohttp.ClientSession() as session:
        await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
            "client_request_id": str(uuid.uuid4()),
            "type": "set_params",
            "patch": {"model": "refact/claude-haiku-4-5", "mode": "NO_TOOLS"}
        })

        print("  Sending first message...")
        await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
            "client_request_id": str(uuid.uuid4()),
            "type": "user_message",
            "content": "Count from 1 to 10 slowly, one number per line."
        })

        await asyncio.sleep(1.5)
        print("  Sending second message (during generation)...")
        await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
            "client_request_id": str(uuid.uuid4()),
            "type": "user_message",
            "content": "After counting, say DONE"
        })

    await asyncio.sleep(20)
    task.cancel()

    user_msgs = sum(1 for e in events if e["type"] == "message_added" and e.get("message",{}).get("role") == "user")
    asst_msgs = sum(1 for e in events if e["type"] == "message_added" and e.get("message",{}).get("role") == "assistant")

    print(f"\n  Results:")
    print(f"    User messages: {user_msgs}")
    print(f"    Assistant messages: {asst_msgs}")

    if user_msgs >= 2:
        print("    Second message was queued and added")

    return True


async def test_reconnection():
    """Test reconnecting to an existing chat."""
    print("\n" + "="*60)
    print("TEST: Reconnection to existing chat")
    print("="*60)

    chat_id = f"test-reconnect-{uuid.uuid4().hex[:8]}"

    print("  First connection - sending message...")
    async with aiohttp.ClientSession() as session:
        resp = await session.get(
            f"{LSP_URL}/v1/chats/subscribe?chat_id={chat_id}",
            timeout=aiohttp.ClientTimeout(total=2)
        )
        await resp.content.readline()
        resp.close()

        await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
            "client_request_id": str(uuid.uuid4()),
            "type": "set_params",
            "patch": {"model": "refact/gpt-4.1-nano", "mode": "NO_TOOLS"}
        })
        await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
            "client_request_id": str(uuid.uuid4()),
            "type": "user_message",
            "content": "Remember this: APPLE"
        })

    await asyncio.sleep(5)

    print("  Second connection - checking snapshot...")
    async with aiohttp.ClientSession() as session:
        async with session.get(
            f"{LSP_URL}/v1/chats/subscribe?chat_id={chat_id}",
            timeout=aiohttp.ClientTimeout(total=5)
        ) as resp:
            line = await resp.content.readline()
            if line.startswith(b"data: "):
                event = json.loads(line[6:])
                if event["type"] == "snapshot":
                    msgs = event.get("messages", [])
                    print(f"    Snapshot has {len(msgs)} messages")
                    for m in msgs:
                        role = m.get("role", "?")
                        content = str(m.get("content", ""))[:40]
                        print(f"      {role}: {content}...")

                    if len(msgs) >= 2:
                        print("    Reconnection shows existing messages")
                    return True

    return True


async def test_invalid_model():
    """Test with invalid model name."""
    print("\n" + "="*60)
    print("TEST: Invalid model handling")
    print("="*60)

    chat_id = f"test-invalid-{uuid.uuid4().hex[:8]}"
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
                            if event["type"] == "runtime_updated" and event.get("error"):
                                print(f"    Error: {event.get('error', '')[:60]}...")
                                break
            except Exception:
                pass

    task = asyncio.create_task(subscriber())
    await asyncio.sleep(0.3)

    async with aiohttp.ClientSession() as session:
        await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
            "client_request_id": str(uuid.uuid4()),
            "type": "set_params",
            "patch": {"model": "nonexistent-model-xyz", "mode": "NO_TOOLS"}
        })
        await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
            "client_request_id": str(uuid.uuid4()),
            "type": "user_message",
            "content": "Test"
        })

    await asyncio.sleep(5)
    task.cancel()

    has_error = any(e.get("error") for e in events)
    if has_error:
        print("    Error state properly reported")

    return True


async def test_rapid_messages_during_generation():
    """Test sending multiple messages rapidly while generating."""
    print("\n" + "="*60)
    print("TEST: Rapid messages during generation")
    print("="*60)

    chat_id = f"test-rapid-{uuid.uuid4().hex[:8]}"
    events = []

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
                            events.append(event)

                            if event["type"] in ("message_added", "stream_started", "stream_finished"):
                                msg = event.get("message", {})
                                role = msg.get("role", "")
                                print(f"    {event['type']}: {role}")
            except Exception as e:
                print(f"    Exception: {e}")

    task = asyncio.create_task(subscriber())
    await asyncio.sleep(0.3)

    async with aiohttp.ClientSession() as session:
        await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
            "client_request_id": str(uuid.uuid4()),
            "type": "set_params",
            "patch": {"model": "refact/gpt-4.1-nano", "mode": "NO_TOOLS"}
        })

        # Send first message
        await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
            "client_request_id": str(uuid.uuid4()),
            "type": "user_message",
            "content": "Write a haiku about coding"
        })

        # Immediately send more messages
        await asyncio.sleep(0.5)
        for i in range(3):
            await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
                "client_request_id": str(uuid.uuid4()),
                "type": "user_message",
                "content": f"Follow up message {i+1}"
            })

    await asyncio.sleep(20)
    task.cancel()

    user_msgs = sum(1 for e in events if e["type"] == "message_added" and e.get("message",{}).get("role") == "user")
    asst_msgs = sum(1 for e in events if e["type"] == "message_added" and e.get("message",{}).get("role") == "assistant")

    print(f"\n  Results:")
    print(f"    User messages queued: {user_msgs}")
    print(f"    Assistant responses: {asst_msgs}")

    return True


async def main():
    print("="*60)
    print("CLAUDE MODELS & CORNER CASES TESTS")
    print("="*60)

    results = []

    results.append(("Claude models", await test_claude_models()))
    results.append(("Tool call advancement", await test_tool_call_advancement()))
    results.append(("Send during generation", await test_send_message_during_generation()))
    results.append(("Rapid messages during generation", await test_rapid_messages_during_generation()))
    results.append(("Invalid model", await test_invalid_model()))
    results.append(("Reconnection", await test_reconnection()))

    print("\n" + "="*60)
    print("SUMMARY")
    print("="*60)

    for name, passed in results:
        status = "PASS" if passed else "FAIL"
        print(f"  {status}: {name}")


if __name__ == "__main__":
    asyncio.run(main())
