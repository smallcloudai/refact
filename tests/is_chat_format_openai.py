import asyncio
import json
from os import getenv
from typing import AsyncIterator, Dict

import aiohttp
from openai import AsyncOpenAI


messages = [
    {"role": "system", "content": "You are a coding assistant"},
    {"role": "user", "content":  "Who are you?"},
    {"role": "system", "content": "I am a coding assistant"},
    {"role": "user", "content": "What are the 3 laws of robotics?"},
]


async def ask_openai(stream: bool = False) -> AsyncIterator[Dict]:
    client = AsyncOpenAI()
    response = await client.chat.completions.create(
        model="gpt-4-turbo", messages=messages,
        stream=stream,
        max_tokens=64
    )
    async for r in response:
        yield r.json()


async def ask_refact(stream: bool = False) -> AsyncIterator[Dict]:
    # aclient = AsyncOpenAI(
    #     base_url="http://127.0.0.1:8001/v1",
    #     api_key=getenv("OPENAI_API_KEY"),
    # )
    # response = await aclient.chat.completions.create(
    #     model="gpt-4", messages=messages,
    #     stream=stream,
    #     max_tokens=64
    # )
    # async for r in response:
    #     yield r.json()
    async with aiohttp.ClientSession() as session:
        async with session.post("http://127.0.0.1:8001/v1/chat/completions", json={
            "model": "gpt-4",
            "messages": messages,
            "stream": stream,
            "max_tokens": 64
        }) as response:
            async for line in response.content:
                yield line


async def execute():
    async for r in ask_refact(True):
        print(r)


def main():
    loop = asyncio.get_event_loop()
    loop.run_until_complete(execute())


if __name__ == "__main__":
    main()

