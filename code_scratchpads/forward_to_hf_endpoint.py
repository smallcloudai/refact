import asyncio
import aiohttp
import time
import os
import json
from typing import Dict, Any


async def real_work(
    model_name: str,
    prompt: str,
    sampling_parameters: Dict[str, Any],
    stream: bool,
):
    hf_token = os.environ["HUGGINGFACE_TOKEN"]
    # yield [{"generated_text": "Hello"}]
    # yield [{"generated_text": " World"}]
    # yield [{"generated_text": "!"}]
    session = aiohttp.ClientSession()
    session.headers.update({"Content-Type": "application/json"})
    session.headers.update({"Authorization": "Bearer " + hf_token})
    modelIdOrEndpoint = "bigcode/starcoder"
    url = "https://api-inference.huggingface.co/models/" + modelIdOrEndpoint
    data = {
        "inputs": prompt,
        "parameters": sampling_parameters,
    }
    if stream:
        data["stream"] = True
    try:
        t0 = time.time()
        async with session.post(url, json=data) as response:
            async for byteline in response.content:
                txt = byteline.decode("utf-8").strip()
                if not txt.startswith("data:"):
                    continue
                txt = txt[5:]
                print("-"*20, "line", "-"*20, "%0.2fms" % ((time.time() - t0) * 1000))
                print(txt)
                print("-"*20, "/line", "-"*20)
                line = json.loads(txt)
                yield line

            # xxx = await response.text()
            # if response.status == 200:
            #     response_json = await response.json()
            #     yield response_json
            # else:
            #     print(response.status)
            #     print(await response.text())
    finally:
        await session.close()


async def test_hf_works_or_not():
    test_code = "def hello_world():\n    \"\"\"\n    This prints the message \"Hello, World!\".\n    \"\"\""
    hf_token = os.environ["HUGGINGFACE_TOKEN"]
    session = aiohttp.ClientSession()
    session.headers.update({"Content-Type": "application/json"})
    session.headers.update({"Authorization": "Bearer " + hf_token})
    modelIdOrEndpoint = "bigcode/starcoder"
    url = "https://api-inference.huggingface.co/models/" + modelIdOrEndpoint
    try:
        inputs = "<fim_prefix>" + test_code + "<fim_suffix><fim_middle>"
        parameters = {
            "max_new_tokens": 60,
            "temperature": 0.2,
            "do_sample": True,
            "top_p": 0.95,
            "stop": ["<|endoftext|>"],
            "return_full_text": False,
            "num_return_sequences": 2,
        }
        stream = True
        data = {
            "inputs": inputs,
            "parameters": parameters,
            "stream": stream,
        }
        for _ in range(2):
            t1 = time.time()
            if stream:
                async with session.post(url, json=data) as response:
                    async for byteline in response.content:
                        txt = byteline.decode("utf-8").strip()
                        if not txt.startswith("data:"):
                            continue
                        print(txt)
            else:
                async with session.post(url, json=data) as response:
                    response_json = await response.json()
                    print(response_json)
            t2 = time.time()
            print("%0.2fms" % (1000 * (t2 - t1)))
            # Not streaming:
            # [{'generated_text': '\n    print("Hello, World!")\n<|endoftext|>'},
            #  {'generated_text': '\n    print("Hello, World!")\n<|endoftext|>'}]
            # Streaming:
            # data: {"token": {"id": 5093, "text": " comment", "logprob": 0.0, "special": false},
            # "generated_text": null, "details": null}
    finally:
        await session.close()


if __name__=="__main__":
    loop = asyncio.get_event_loop()
    loop.run_until_complete(test_hf_works_or_not())
