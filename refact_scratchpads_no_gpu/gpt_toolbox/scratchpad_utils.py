from typing import Tuple


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
