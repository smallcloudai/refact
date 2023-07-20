from itertools import zip_longest

from refact_encoding import RefactEncoding

from typing import Tuple


def trim_context_infill(
        prefix: str,
        suffix: str,
        enc: RefactEncoding,
        tokens_limit: int
) -> Tuple[str, str]:
    lines_prefix = [(l, 'prefix') for l in reversed(prefix.splitlines(keepends=True))]
    lines_suffix = [(l, 'suffix') for l in suffix.splitlines(keepends=True)]

    merged_lines = [val for pair in zip_longest(lines_prefix, lines_suffix) for val in pair if val]

    lines_prefix_p, lines_suffix_p = [], []
    for line, t in merged_lines:
        if (line_tok_cnt := len(enc.encode(line))) >= tokens_limit: break
        lines_prefix_p.append(line) if t == 'prefix' else lines_suffix_p.append(line)
        tokens_limit -= line_tok_cnt

    prefix = ''.join(reversed(lines_prefix_p))
    suffix = ''.join(lines_suffix_p)
    return prefix, suffix


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
