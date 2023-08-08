from refact_lsp import refact_lsp_server
from refact_lsp import refact_client
import asyncio
import aiohttp
import os
import termcolor
from typing import Dict, Optional


async def regular_code_completion(
    files: Dict[str, str],
    cursor_file: str,
    cursor: int,
    max_tokens: int,
    multiline: bool,
    temperature: Optional[float] = None,
):
    if "SMALLCLOUD_API_KEY" not in os.environ:
        raise ValueError("Please either set SMALLCLOUD_API_KEY environment variable or create requests session manually.")
    sess = aiohttp.ClientSession(headers={
        "Authorization": "Bearer %s" % os.environ["SMALLCLOUD_API_KEY"],
    })
    for fn, txt in files.items():
        if not txt.endswith("\n"):
            # server side will add it anyway, add here for comparison to work correctly later in this function
            files[fn] += "\n"
    try:
        ans = await refact_client.nlp_model_call(
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
            verbose=2,
        )
    finally:
        await sess.close()
    # print(ans)
    # print(ans["choices"][0]["files"][cursor_file])

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
    # import pudb; pudb.set_trace()
    return modif_file[cursor : len(modif_file) + stop_at]


async def test_multiline(no_newline_in_the_end: bool):
    hello_world_py = "# This print hello world and does not do anything else\ndef hello_world():"
    if not no_newline_in_the_end:
        hello_world_py += "\n"
    files = {
        "hello_world.py": hello_world_py,
    }
    completion = await regular_code_completion(
        files,
        "hello_world.py",
        len(hello_world_py),
        50,
        multiline=True,
    )
    print(termcolor.colored(hello_world_py, "yellow") + termcolor.colored(str(completion), "green"))
    print("checking if correct \"%s\"" % str(completion).replace("\n", "\\n"))
    assert completion.strip().lower().replace("!", "") in [
        "print('hello world')",
        "print(\"hello world\")",
    ]


async def test_everything():
    await test_multiline(False)
    await test_multiline(True)


def main():
    # print("listening on 127.0.0.1:1337")
    # refact_lsp_server.server.start_tcp("127.0.0.1", 1337)

    loop = asyncio.new_event_loop()
    try:
        loop.run_until_complete(test_everything())
    finally:
        loop.close()


# TODO:
# * allow empty model
# * allow no temperature
# * /contrast should return mime type json

