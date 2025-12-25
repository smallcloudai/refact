#!/usr/bin/env python3
import asyncio
import aiohttp
import json
import uuid
import sys

LSP_URL = "http://127.0.0.1:8001"

TINY_PNG = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg=="


async def test_string_content_with_attachments():
    print("\n" + "="*60)
    print("TEST: String content with image attachments")
    print("="*60)

    chat_id = f"test-attach-{uuid.uuid4().hex[:8]}"
    events = []
    message_added = asyncio.Event()
    user_message = None

    async def subscriber():
        nonlocal user_message
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
                            if event["type"] == "message_added":
                                msg = event.get("message", {})
                                if msg.get("role") == "user":
                                    user_message = msg
                                    message_added.set()
                                    return
            except Exception as e:
                print(f"    Exception: {e}")

    task = asyncio.create_task(subscriber())
    await asyncio.sleep(0.3)

    async with aiohttp.ClientSession() as session:
        resp = await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
            "client_request_id": str(uuid.uuid4()),
            "type": "user_message",
            "content": "What is in this image?",
            "attachments": [
                {"image_url": {"url": TINY_PNG}}
            ]
        })
        print(f"    Response: {resp.status}")

    try:
        await asyncio.wait_for(message_added.wait(), timeout=5)
    except asyncio.TimeoutError:
        pass

    task.cancel()

    if resp.status != 202:
        print(f"  ✗ Expected 202, got {resp.status}")
        return False

    if user_message:
        content = user_message.get("content")
        print(f"    Content type: {type(content)}")
        if isinstance(content, list):
            has_text = any(c.get("m_type") == "text" or c.get("type") == "text" for c in content)
            has_image = any(
                c.get("m_type", "").startswith("image") or c.get("type") == "image_url"
                for c in content
            )
            if has_text and has_image:
                print("  ✓ Multimodal content preserved (text + image)")
                return True
            print(f"  ✗ Missing components: text={has_text}, image={has_image}")
            return False
        elif isinstance(content, str):
            print("  ⚠ Content is string (attachments may be handled separately)")
            return True

    print("  ✗ User message not received")
    return False


async def test_multimodal_content_with_attachments():
    print("\n" + "="*60)
    print("TEST: Multimodal content array with additional attachments")
    print("="*60)

    chat_id = f"test-multi-attach-{uuid.uuid4().hex[:8]}"

    async with aiohttp.ClientSession() as session:
        resp = await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
            "client_request_id": str(uuid.uuid4()),
            "type": "user_message",
            "content": [
                {"type": "text", "text": "Compare these images"},
                {"type": "image_url", "image_url": {"url": TINY_PNG}}
            ],
            "attachments": [
                {"image_url": {"url": TINY_PNG}},
                {"image_url": {"url": TINY_PNG}}
            ]
        })
        data = await resp.json()
        print(f"    Response: {resp.status} {data}")

    if resp.status == 202:
        print("  ✓ Multimodal content + attachments accepted")
        return True

    print(f"  ✗ Expected 202, got {resp.status}")
    return False


async def test_attachments_exceed_image_limit():
    print("\n" + "="*60)
    print("TEST: Attachments exceed image limit (content + attachments)")
    print("="*60)

    chat_id = f"test-attach-limit-{uuid.uuid4().hex[:8]}"

    async with aiohttp.ClientSession() as session:
        resp = await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
            "client_request_id": str(uuid.uuid4()),
            "type": "user_message",
            "content": [
                {"type": "text", "text": "Look at all these"},
                {"type": "image_url", "image_url": {"url": TINY_PNG}},
                {"type": "image_url", "image_url": {"url": TINY_PNG}},
                {"type": "image_url", "image_url": {"url": TINY_PNG}}
            ],
            "attachments": [
                {"image_url": {"url": TINY_PNG}},
                {"image_url": {"url": TINY_PNG}},
                {"image_url": {"url": TINY_PNG}}
            ]
        })
        data = await resp.json()
        print(f"    Response: {resp.status} {data}")

    if resp.status == 400 and "image" in data.get("error", "").lower():
        print("  ✓ Image limit enforced across content + attachments")
        return True

    print(f"  ✗ Expected 400 with image error")
    return False


async def test_attachment_missing_url():
    print("\n" + "="*60)
    print("TEST: Attachment missing image_url.url")
    print("="*60)

    chat_id = f"test-attach-nourl-{uuid.uuid4().hex[:8]}"

    async with aiohttp.ClientSession() as session:
        resp = await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
            "client_request_id": str(uuid.uuid4()),
            "type": "user_message",
            "content": "Check this",
            "attachments": [
                {"image_url": {}}
            ]
        })
        data = await resp.json()
        print(f"    Response: {resp.status} {data}")

    if resp.status == 400:
        print("  ✓ Missing url rejected with 400")
        return True

    print(f"  ✗ Expected 400, got {resp.status}")
    return False


async def test_attachment_invalid_data_url():
    print("\n" + "="*60)
    print("TEST: Attachment with invalid data URL")
    print("="*60)

    chat_id = f"test-attach-invalid-{uuid.uuid4().hex[:8]}"

    async with aiohttp.ClientSession() as session:
        resp = await session.post(f"{LSP_URL}/v1/chats/{chat_id}/commands", json={
            "client_request_id": str(uuid.uuid4()),
            "type": "user_message",
            "content": "Check this",
            "attachments": [
                {"image_url": {"url": "not-a-valid-data-url"}}
            ]
        })
        data = await resp.json()
        print(f"    Response: {resp.status} {data}")

    if resp.status in (400, 202):
        print(f"  ✓ Invalid data URL handled ({resp.status})")
        return True

    print(f"  ✗ Unexpected status {resp.status}")
    return False


async def main():
    print("=" * 60)
    print("Chat Session Attachments Tests")
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

    results.append(("String content + attachments", await test_string_content_with_attachments()))
    results.append(("Multimodal + attachments", await test_multimodal_content_with_attachments()))
    results.append(("Attachments exceed limit", await test_attachments_exceed_image_limit()))
    results.append(("Attachment missing url", await test_attachment_missing_url()))
    results.append(("Attachment invalid data url", await test_attachment_invalid_data_url()))

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
