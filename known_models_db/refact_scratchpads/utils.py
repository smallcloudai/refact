import re
from itertools import zip_longest

import torch as th
from typing import Tuple

from encoding_wrapper.refact_encoding import RefactEncoding


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


def temperature_top_k_top_p_filtering(logits, temperature=1, top_k=0, top_p=0, filter_value=-float('Inf')):
    assert logits.dim() == 1

    temperature = min(temperature, 1.0)
    temperature = max(temperature, 0.0)
    logits = logits / (temperature + 0.01)
    top_k = min(top_k, logits.size(-1))

    if top_k > 0:
        indices_to_remove = logits < th.topk(logits, top_k)[0][..., -1, None]
        logits = logits.masked_fill(indices_to_remove, filter_value)

    if 0.0 < top_p < 1.0:
        sorted_logits, sorted_indices = th.sort(logits, descending=True)
        cumulative_probs = sorted_logits.softmax(dim=-1).cumsum(dim=-1)

        sorted_indices_to_remove = cumulative_probs > top_p
        sorted_indices_to_remove[..., 1:] = sorted_indices_to_remove[..., :-1].clone()
        sorted_indices_to_remove[..., 0] = 0

        indices_to_remove = sorted_indices_to_remove.scatter(0, sorted_indices, sorted_indices_to_remove)
        logits = logits.masked_fill(indices_to_remove, filter_value)
    return logits


def simple_stoplist_cut(orig: str, dest: str, head: int, tail: int) -> str:
    expanded_head = orig.rfind("\n", 0, head) + 1
    result = []
    for idx, line in enumerate(dest[expanded_head:len(dest)-tail].splitlines(keepends=True)):
        re_patterns = "|".join([
            r"copyright", r"copyleft", r"(C)", r"©", r"author", r"license",
            r'[\w.+-]+@[\w-]+\.[\w.-]+',  # email
        ])
        for _ in re.finditer(re_patterns, line.lower()):
            return "".join(result)
        result.append(line if idx > 0 else line[head - expanded_head:])
    return "".join(result)
