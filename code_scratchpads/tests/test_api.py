import requests
import termcolor
import json
import os


hello_world = "def hello_world():\n    '''\n    This function prints 'Hello World' and returns True.\n    '''\n"


def call_completion(
    code,
    *,
    model,
    cursor_line,
    cursor_character,
    stream,
    multiline,
):
    headers = {
        "Content-Type": "application/json",
        "Authorization": "Bearer %s" % os.environ.get("HF_TOKEN"),
        }
    r = requests.post(
        "http://127.0.0.1:8001/v1/code-completion",
        json={
            "inputs": {
                "sources": {"test.py": code},
                "cursor": {
                    "file": "test.py",
                    "line": cursor_line,
                    "character": cursor_character,
                },
                "multiline": multiline,
            },
            "parameters": {
                # "temperature": 0.8,
            },
            "model": model,
            # "scratchpad": "FIM-PSM",
            "stream": stream,
        },
        headers=headers,
    )
    if r.status_code != 200:
        raise ValueError("Unexpected response\n%s" % r.text)
    if not stream:
        resp = r.json()
        if isinstance(resp, list):
            return resp[0]["code_completion"]
    else:
        accum = ""
        for line in r.iter_lines():
            txt = line.decode("utf-8").strip()
            if not txt.startswith("data:"):
                continue
            txt = txt[5:].strip()
            if txt == "[DONE]":
                break
            j = json.loads(txt)
            accum += j["code_completion_delta"]
        return accum


def pretty_print_wrapper(
    code,
    *,
    multiline,
    cursor_line,
    cursor_character,
    **kwargs
):
    print("-"*100)
    for line_n, line in enumerate(code.splitlines()):
        if line_n == cursor_line:
            print("%s" % termcolor.colored(line[:cursor_character], "green") + "|" + termcolor.colored(line[cursor_character:], "green"))
        else:
            print("%s" % termcolor.colored(line, "green"))
    ans = call_completion(code, multiline=multiline, cursor_line=cursor_line, cursor_character=cursor_character, **kwargs)
    print("multiline=%s, completion \"%s\"" % (multiline, termcolor.colored(ans.replace("\n", "\\n"), "cyan")))
    return ans


def test_battery(model, stream):
    x = pretty_print_wrapper(hello_world, model=model, stream=stream, multiline=False, cursor_line=2, cursor_character=52)
    assert x.rstrip() in ["rue", "rue."], x
    x = pretty_print_wrapper(hello_world, model=model, stream=stream, multiline=False, cursor_line=3, cursor_character=7)
    assert x == "", x
    x = pretty_print_wrapper(hello_world, model=model, stream=stream, multiline=True, cursor_line=3, cursor_character=7)
    assert x.rstrip() == "\n    print('Hello World')\n    return True", x
    x = pretty_print_wrapper(hello_world + "    \n", model=model, stream=stream, multiline=True, cursor_line=4, cursor_character=4)
    assert x.rstrip() == "print('Hello World')\n    return True", x
    x = pretty_print_wrapper(hello_world.replace("hello_world", ""), model=model, stream=stream, multiline=True, cursor_line=0, cursor_character=4)
    assert x.rstrip() in ["hello_world", "print_hello"], x
    x = pretty_print_wrapper(hello_world.replace("hello_world", ""), model=model, stream=stream, multiline=False, cursor_line=0, cursor_character=4)
    assert x.rstrip() in ["hello_world():", "print_hello():"], x


if __name__ == "__main__":
    test_battery("bigcode/starcoder", stream=False)
