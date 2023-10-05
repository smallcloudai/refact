import asyncio
import aiohttp
import time
import os


HUGGINGFACE_TOKEN = os.environ["HUGGINGFACE_TOKEN"]


async def minimal_hf_endpoint_test():
    test_code = "def hello_world():\n    \"\"\"\n    This prints the message \"Hello, World!\" and returns True.\n    \"\"\""
    session = aiohttp.ClientSession()
    session.headers.update({"Content-Type": "application/json"})
    session.headers.update({"Authorization": "Bearer " + HUGGINGFACE_TOKEN})
    # modelIdOrEndpoint = "bigcode/tiny_starcoder_py"
    modelIdOrEndpoint = "bigcode/starcoder"
    url = "https://api-inference.huggingface.co/models/" + modelIdOrEndpoint
    try:
        inputs = "<fim_prefix>" + test_code + "<fim_suffix><fim_middle>"
        parameters = {
            "max_new_tokens": 60,
            "temperature": 0.2,
            "do_sample": True,
            "top_p": 0.95,
            # "stop": ["<|endoftext|>"],    # "\n   " is a StarCoder token that can stop on the first \n
            "return_full_text": False,
            "num_return_sequences": 2,
        }
        stream = False
        data = {
            "inputs": inputs,
            "parameters": parameters,
            "stream": stream,
        }
        for attempt in range(2):
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
            print("attempt %d, completed in %0.2fms" % (attempt + 1, 1000 * (t2 - t1)))
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
    loop.run_until_complete(minimal_hf_endpoint_test())
