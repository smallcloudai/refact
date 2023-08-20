import asyncio
import aiohttp
import time
import os
import json
import logging
from typing import Dict, Any, Optional


logger = logging.getLogger("HF_CLIENT")


_reuse_session: Optional[aiohttp.ClientSession] = None


def global_hf_session_get():
    global _reuse_session
    if _reuse_session is None:
        _reuse_session = aiohttp.ClientSession()
        _reuse_session.headers.update({"Content-Type": "application/json"})
    return _reuse_session


def global_hf_session_close():
    global _reuse_session
    if _reuse_session is not None:
        _reuse_session.close()
    _reuse_session = None


async def real_work(
    model_name: str,
    prompt: str,
    sampling_parameters: Dict[str, Any],
    stream: bool,
    auth_from_client: Optional[str],
):
    session = global_hf_session_get()
    url = "https://api-inference.huggingface.co/models/" + model_name
    headers = {
        "Authorization": "Bearer " + (auth_from_client or os.environ["HUGGINGFACE_TOKEN"]),
    }
    data = {
        "inputs": prompt,
        "parameters": sampling_parameters,
        "stream": stream,
    }
    t0 = time.time()
    if stream:
        async with session.post(url, json=data, headers=headers) as response:
            async for byteline in response.content:
                txt = byteline.decode("utf-8").strip()
                if not txt.startswith("data:"):
                    continue
                txt = txt[5:]
                # print("-"*20, "line", "-"*20, "%0.2fms" % ((time.time() - t0) * 1000))
                # print(txt)
                # print("-"*20, "/line", "-"*20)
                line = json.loads(txt)
                yield line
    else:
        async with session.post(url, json=data) as response:
            response_txt = await response.text()
            if response.status == 200:
                response_json = json.loads(response_txt)
                yield response_json
            else:
                logger.warning("http status %s, response text was:\n%s" % (response.status, response_txt))


async def test_hf_works_or_not():
    test_code = "def hello_world():\n    \"\"\"\n    This prints the message \"Hello, World!\" and returns True.\n    \"\"\""
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
            "stop": ["<|endoftext|>", "\n   "],
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
