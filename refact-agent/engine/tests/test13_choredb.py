import aiohttp
import asyncio
import termcolor
import json
import argparse
import time
from typing import Dict, List
from refact import chat_client


BASE_URL = "http://127.0.0.1:8001"

silly_message = {
    "role": "user",
    "content": "Why did the scarecrow win an award? Because he was outstanding in his field!",
}


async def listen_to_cthreads(session):
    async with session.post(
        f"{BASE_URL}/db_v1/cthreads-sub",
        json={"limit": 100, "quicksearch": ""},
        headers={"Content-Type": "application/json"},
    ) as response:
        if response.status != 200:
            print(termcolor.colored(f"Failed to connect to SSE. Status code: {response.status}", "red"))
            return
        print(termcolor.colored("Connected to SSE", "green"))
        async for line in response.content:
            if line:
                decoded_line = line.decode('utf-8').strip()
                if decoded_line:
                    print(termcolor.colored(decoded_line, "yellow"))
                    print()

async def listen_to_cmessages(session, thread_id):
    async with session.post(
        f"{BASE_URL}/db_v1/cmessages-sub",
        json={"cmessage_belongs_to_cthread_id": thread_id},
        headers={"Content-Type": "application/json"},
    ) as response:
        if response.status != 200:
            print(termcolor.colored(f"Failed to connect to SSE. Status code: {response.status}", "red"))
            return
        print(termcolor.colored("Connected to SSE %s" % thread_id, "green"))
        async for line in response.content:
            if line:
                decoded_line = line.decode('utf-8').strip()
                if decoded_line:
                    print(termcolor.colored(decoded_line, "yellow"))
                    print()


async def various_updates_generator(session, n, cthread_id):
    r = await session.post(f"{BASE_URL}/db_v1/cthread-update", json={
        "cthread_id": cthread_id,
        "cthread_title": "Frog launcher thread %d" % n,
        "cthread_anything_new": False,
        "cthread_created_ts": time.time(),
        "cthread_model": "gpt-4o-mini",
        "cthread_temperature": 0.8,
        "cthread_max_new_tokens": 2048,
        "cthread_n": 1,
        "cthread_error": ("pause" if n != 2 else ""),
    })
    assert r.status == 200, f"oops:\n{r}"
    msg: List[chat_client.Message] = [
        chat_client.Message(role="user", content="Hello mister assistant, I have a question for you"),
        chat_client.Message(role="user", content="Find Frog in this project"),
    ]
    messages_payload = []
    for mi, m in enumerate(msg):
        messages_payload.append({
            "cmessage_belongs_to_cthread_id": cthread_id,
            "cmessage_alt": 0,
            "cmessage_num": mi,
            "cmessage_prev_alt": mi - 1,
            "cmessage_json": json.dumps(m.model_dump(exclude_unset=True)),
        })
    r = await session.post(f"{BASE_URL}/db_v1/cmessages-update", json=messages_payload)
    assert r.status == 200, f"oops:\n{r}"
    print(termcolor.colored("updates over %s" % cthread_id, "green"))


async def main(only_sub=False, only_update=False):
    cthread_id = f"test13thread{int(time.time())}"
    async with aiohttp.ClientSession() as session:
        tasks = []
        if only_sub or (not only_sub and not only_update):
            tasks.append(asyncio.create_task(listen_to_cthreads(session)))
            tasks.append(asyncio.create_task(listen_to_cmessages(session, cthread_id)))
        if only_update or (not only_sub and not only_update):
            tasks.append(asyncio.create_task(various_updates_generator(session, 1, cthread_id + "_1")))
            tasks.append(asyncio.create_task(various_updates_generator(session, 2, cthread_id + "_2")))
            tasks.append(asyncio.create_task(various_updates_generator(session, 3, cthread_id + "_3")))
        await asyncio.gather(*tasks)

    print(termcolor.colored("\nTEST OVER", "green", attrs=["bold"]))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="ChoreDB test script")
    parser.add_argument("--only-sub", action="store_true", help="Run only the subscription part")
    parser.add_argument("--only-update", action="store_true", help="Run only the update part")
    args = parser.parse_args()

    asyncio.run(main(only_sub=args.only_sub, only_update=args.only_update))
