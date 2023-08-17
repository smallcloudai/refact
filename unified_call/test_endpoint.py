import asyncio
import aiohttp
import time


token = "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"


test_code = "def hello_world():\n    \"\"\"\n    This prints the message \"Hello, World!\".\n    \"\"\""


async def test_run(session, url):
    inputs = "<fim_prefix>" + test_code + "<fim_suffix><fim_middle>"
    parameters = {
        "max_new_tokens": 60,
        "temperature": 0.2,
        "do_sample": True,
        "top_p": 0.95,
        "stop": ["<|endoftext|>"],
        "echo": False,
    }
    data = {
        "inputs": inputs,
        "parameters": parameters,
    }

    for _ in range(2):
        t1 = time.time()
        async with session.post(url, json=data) as response:
            response_json = await response.json()
        t2 = time.time()
        print("%0.2fms" % (1000 * (t2 - t1)), response_json)


async def main():
    session = aiohttp.ClientSession()
    session.headers.update({"Content-Type": "application/json"})
    session.headers.update({"Authorization": "Bearer " + token})
    modelIdOrEndpoint = "bigcode/starcoder"
    url = "https://api-inference.huggingface.co/models/" + modelIdOrEndpoint

    try:
        await test_run(session, url)
    finally:
        await session.close()


if __name__=="__main__":
    loop = asyncio.get_event_loop()
    loop.run_until_complete(main())
