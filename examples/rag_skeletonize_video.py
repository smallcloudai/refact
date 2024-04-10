import os
import random

import requests
import termcolor

from pathlib import Path

from pygments import highlight
from pygments.lexers import PythonLexer
from pygments.formatters import TerminalFormatter


fpath = os.path.join("tests", "emergency_frog_situation", "jump_to_conclusions.py")


SPECIAL_TOKENS = ["<fim_prefix>", "<fim_suffix>", "<fim_middle>"]


def code_completion_prompt_with_rag(rag_token_limit):
    sample_code = open(fpath, "r").read()
    CURSOR_AT = "W, H"
    lines = sample_code.split("\n")
    cursor_line_n = sample_code[:sample_code.find(CURSOR_AT)].count("\n");
    cursor_line_str = lines[cursor_line_n]
    cursor_column = cursor_line_str.find(CURSOR_AT)
    response = requests.post(
        url="http://127.0.0.1:8001/v1/code-completion-prompt",
        json={
            "inputs": {
                "sources": {
                    fpath: sample_code.replace(CURSOR_AT, ""),
                },
                "cursor": {
                    "file": fpath,
                    "line": cursor_line_n,
                    "character": cursor_column,
                },
                "multiline": True
            },
            "use_ast": True,
            "rag_tokens_n": rag_token_limit,
        },
        headers={
            "Content-Type": "application/json",
        },
    )
    prompt = response.json()["prompt"]
    # print(prompt)
    i = prompt.index("<fim_prefix>")
    aaaa = prompt[:i]
    aaaa = aaaa.split("<file_sep>")[1]
    lines_n_in_aaaa = aaaa.count("\n") + 1
    print(termcolor.colored(aaaa, "yellow"))
    print("\n" * (30 - lines_n_in_aaaa))
    # for token in SPECIAL_TOKENS:
    #     prompt = prompt.replace(token, termcolor.colored(token, "blue"))
    # prompt = prompt.replace("\n".join(prefix_lines), termcolor.colored("\n".join(prefix_lines), "yellow"))
    # middle = termcolor.colored(middle, 'green')
    # print(f"{prompt}{middle}\n\n")


if __name__ == "__main__":
    for x in range(200, 512-30):
        rag_token_limit = 512 - x
        print(x)
        code_completion_prompt_with_rag(rag_token_limit)
