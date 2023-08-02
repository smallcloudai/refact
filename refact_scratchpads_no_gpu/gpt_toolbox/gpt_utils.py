import re
from itertools import zip_longest
from typing import Dict, Optional, Tuple

import tiktoken


def msg(role: str, content: str) -> Dict[str, str]:
    assert role in ['system', 'user', 'assistant']
    return {'role': role, 'content': content}


def full_line_selection(cursor0: int, cursor1: int, txt: str) -> Tuple[int, int, str]:
    """
    Adjusts selection to only include full lines.
    """
    c0, c1, buff = '<|cursor0|>', '<|cursor1|>', ''
    txt: str = txt[:cursor0] + c0 + txt[cursor0:cursor1] + c1 + txt[cursor1:]

    lines_new = []
    for line in txt.split('\n'):
        if buff:
            line = buff + line
            buff = ''
        if c0 in line:
            if not line.split(c0)[1].strip():
                buff = c0
                line = line.replace(c0, "")
            else:
                line = c0 + line.replace(c0, "")

        if c1 in line:
            if not line.split(c1)[0].strip() and lines_new:
                lines_new[-1] += c1
                line = line.replace(c1, "")
            else:
                line = line.replace(c1, "") + c1
        lines_new.append(line)

    txt_new = '\n'.join(lines_new)
    cursor0 = txt_new.index(c0)
    cursor1 = txt_new.replace(c0, "").index(c1)
    selection = txt_new.replace(c0, "").replace(c1, "")[cursor0:cursor1]

    return cursor0, cursor1, selection


def code_block_postprocess(txt: str) -> str:
    lines_code = []
    is_code = False
    for line in txt.split('\n'):
        if '```' in line:
            is_code = not is_code
            continue
        if is_code:
            lines_code.append(line)

    code = '\n'.join(lines_code) or txt
    return code


def find_substring_positions(substring, text) -> Optional[Tuple[int, int]]:
    words = substring.split()
    pattern = r'\s*'.join(map(re.escape, words))
    match = re.search(pattern, text)
    if not match:
        return

    c0, c1, _ = full_line_selection(match.start(), match.end(), text)
    return c0, c1


def trim_context_tok(
        cursor0: int,
        cursor1: int,
        text: str,
        enc: tiktoken.Encoding,
        max_tokens: int = 2000
) -> Tuple[int, int, str]:
    selection = text[cursor0:cursor1]
    tokens_left = max_tokens - len(enc.encode(selection, disallowed_special=()))

    lines_before = ((l, 'before') for l in reversed(text[:cursor0].splitlines()))
    lines_after = ((l, 'after') for l in text[cursor1:].splitlines())
    merged_lines = [val for pair in zip_longest(lines_before, lines_after) for val in pair if val]

    lines_before_p, lines_after_p = [], []
    for line, t in merged_lines:
        if (line_tok_cnt := len(enc.encode(line, disallowed_special=()))) >= tokens_left: break
        lines_before_p.append(line) if t == 'before' else lines_after_p.append(line)
        tokens_left -= line_tok_cnt

    txt_before = '\n'.join(reversed(lines_before_p)) + '\n'
    txt_after = '\n'.join(lines_after_p)
    txt = txt_before + selection + txt_after
    cursor0, cursor1 = len(txt_before), len(txt_before) + len(selection)

    # print("chars before %i -> cut to %i" % (len(text[:cursor0]), len(txt_before)))
    # print("chars  after %i -> cut to %i" % (len(text[cursor1:]), len(txt_after)))
    # print("before %i bytes -> %i tokens" % (len(txt_before), len(enc.encode(txt_before, disallowed_special=()))))
    # print("after  %i bytes -> %i tokens" % (len(txt_after), len(enc.encode(txt_after, disallowed_special=()))))
    # print("tokens + tokens + tokens = %i" % (len(enc.encode(txt, disallowed_special=()))))

    return cursor0, cursor1, txt

