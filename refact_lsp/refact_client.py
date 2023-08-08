import aiohttp
import time
import os
import json
from typing import Optional, Tuple, Generator, Union, Dict


base_url = "https://inference.smallcloud.ai/v1/"


class APIConnectionError(Exception):
    pass


async def nlp_model_call(
    endpoint: str,
    model: str,
    *,
    req_session: Optional[aiohttp.ClientSession]=None,
    max_tokens: int,
    temperature: Optional[float]=None,
    top_p: Optional[float]=None,
    top_n: Optional[int]=None,
    verbose: int=0,
    **pass_args
) -> Union[Tuple[str, str], Generator[Tuple[str, str], None, None]]:
    """
    A simplified version without streaming
    """
    req_session = req_session or aiohttp.ClientSession()
    assert isinstance(req_session, aiohttp.ClientSession)
    url = base_url + endpoint
    data = {
        "model": model,
        "max_tokens": max_tokens,
        "stream": False,
        **pass_args,
    }
    if top_p is not None:
        data["top_p"] = top_p
    if top_n is not  None:
        data["top_n"] = top_n
    if temperature is not None:
        data["temperature"] = temperature
    if verbose > 1:
        print("POST %s" % (data,))
    resp = None
    txt = ""
    try:
        t0 = time.time()
        resp = await req_session.post(url, json=data)
        t1 = time.time()
        if verbose > 0:
            print("%0.1fms %s" % (1000*(t1 - t0), url))
        txt = await resp.text()
    except Exception as e:
        raise APIConnectionError("completions() failed: %s" % str(e))
    if resp.status != 200:
        raise APIConnectionError("status=%i, server returned:\n%s" % (resp.status, txt))
    try:
        j = json.loads(txt)
    except Exception as e:
        raise APIConnectionError("completions() json parse failed: %s\n%s" % (str(e), txt))
    return j


async def regular_code_completion(
    sess: aiohttp.ClientSession,
    files: Dict[str, str],
    cursor_file: str,
    cursor: int,
    max_tokens: int,
    multiline: bool,
    temperature: Optional[float] = None,
):
    for fn, txt in files.items():
        if not txt.endswith("\n"):
            # server side will add it anyway, add here for comparison to work correctly later in this function
            files[fn] += "\n"
    ans = await nlp_model_call(
        "contrast",
        "CONTRASTcode",
        req_session=sess,
        sources=files,
        intent="Infill",
        function="infill",
        cursor_file=cursor_file,
        cursor0=cursor,
        cursor1=cursor,
        max_tokens=max_tokens,
        temperature=temperature,
        stop=(["\n\n"] if multiline else ["\n"]),
        verbose=1,
    )

    #  Find an \n after any different char, when looking from the end. The goal is to find a line that's different, but a complete line.
    stop_at = None
    i = -1
    whole_file = files[cursor_file]
    modif_file = ans["choices"][0]["files"][cursor_file]
    length = min(len(whole_file), len(modif_file))
    any_different = False
    while i > -length:
        if whole_file[i] == "\n":
            stop_at = i + 1
        if whole_file[i] != modif_file[i]:
            any_different = True
            break
        i -= 1
    fail = cursor >= len(modif_file) + stop_at;
    if fail or not any_different:
        return None
    return modif_file[cursor : len(modif_file) + stop_at]

