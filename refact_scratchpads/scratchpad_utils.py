from itertools import zip_longest

from typing import Tuple


def trim_context_infill(
        prefix: str,
        suffix: str,
        enc,
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
