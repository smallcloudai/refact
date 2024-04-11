import os, time

import requests
import termcolor

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
    i = prompt.index("<fim_prefix>")
    good_for_video = prompt
    good_for_video = prompt[:i].split("<file_sep>")
    if len(good_for_video) <= 1:
        print(prompt)
        return
    good_for_video = "\n".join(good_for_video[:-1])
    good_for_video = good_for_video.replace("<repo_name>default_repo\n", "")
    good_for_video = highlight(good_for_video, PythonLexer(), TerminalFormatter())
    lines_n_in_good_for_video = good_for_video.count("\n") + 1
    # print(termcolor.colored(good_for_video, "yellow"))
    print(good_for_video)
    print("\n" * (50 - lines_n_in_good_for_video))


if __name__ == "__main__":
    for x in range(50, 512-50+1):
        rag_token_limit = 512 - x
        print("rag_token_limit", rag_token_limit)
        code_completion_prompt_with_rag(rag_token_limit)
    time.sleep(10)
