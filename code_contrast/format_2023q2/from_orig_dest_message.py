import random
import numpy as np
from code_contrast.format_2023q2.element import Format2023q2

from cdifflib import CSequenceMatcher

from code_contrast.format_2022q3 import ops_remove_short_equals, ops_stochastic_expand

from typing import List, Dict, Tuple, DefaultDict, Any, Optional

from code_contrast.format_2023q2.packing import Packer
from code_contrast.format_2023q2.el_file import FileElement
from code_contrast.format_2023q2.el_chunk import ChunkElement
from code_contrast.format_2023q2.el_msg import MsgElement


def from_odm_dict(
    fmt: Format2023q2,
    odm: Dict[str, Any],
    for_training = False,
    exact_cx_lines0 = -1,
    exact_cx_lines1 = -1,
    external_poi_ranges: Optional[DefaultDict[str, List[Tuple[int, int]]]] = None,
    want_cursor_token: bool = False,
) -> Tuple[Packer, int]:
    pack = Packer(fmt)
    files1 = list(odm["orig"].keys())
    files2 = list(odm["dest"].keys())
    assert set(files2).issubset(set(files1))
    fns = list(files1)
    if not for_training:
        # The main file assumed is first in odm["orig"].keys()
        # This moves it to the end, more visible to the model
        fns.reverse()
    else:
        random.shuffle(fns)
    files = []
    chunks: List[ChunkElement] = []
    for fn in fns:
        if (external_poi_ranges is None or fn not in external_poi_ranges) and fn not in files2:
            print("WARNING: file '%s' is not in dest or POI, context will not contain it" % fn)
            continue
        f = FileElement(fn, [(x + "\n") for x in odm["orig"][fn].splitlines()])
        pack.add_to_plan(f)
        if external_poi_ranges and fn in external_poi_ranges:
            poi_list = external_poi_ranges[fn]
            for line0, line1 in poi_list:
                f.add_expanding_range(line0, line1, aux=1)
        files.append(f)
    msg = MsgElement("USER", odm["commitmsg"])
    msg_plan_n = pack.add_to_plan(msg)
    for fn, f in zip(fns, files):
        if fn not in odm["dest"]:
            continue
        chunks.extend(_run_diff_for_single_file(f, [(x + "\n") for x in odm["dest"][fn].splitlines()], exact_cx_lines0, exact_cx_lines1))
    random.shuffle(chunks)
    for chunk in chunks:
        pack.add_to_plan(chunk)
    if want_cursor_token and len(chunks) == 1:
        file0 = chunks[0].orig_file
        modlines = file0._lines_deleted | file0._lines_replaced | file0._lines_inspoints
        thischunk_lines = set(range(chunks[0].line_n, chunks[0].line_n + len(chunks[0].to_del) + 1))
        thischunk_modlines = list(thischunk_lines & modlines)
        if len(thischunk_modlines) > 0:  # Can be zero for whatever reason, cursor appearance is random anyway
            aim = random.choice(thischunk_modlines)
            shift = np.random.poisson(2)
            sign = np.random.choice([-1, 1])
            file0._cursor_token_at_line = aim + shift * sign
    return pack, msg_plan_n


def _run_diff_for_single_file(f: FileElement, dest_text: List[str], exact_cx_lines0: int, exact_cx_lines1: int):
    chunks = []
    if len(f.file_lines)==0:
        f.file_lines.append("\n")
    if f.file_lines[-1][-1] != "\n":
        f.file_lines[-1] += "\n"
    if dest_text[-1][-1] != "\n":
        dest_text[-1] += "\n"
    lines_diff = list(CSequenceMatcher(None, f.file_lines, dest_text).get_opcodes())
    f.lines_diff = lines_diff
    for op, i0, i1, _, _ in lines_diff:
        if op == "insert":
            f._lines_inspoints.add(i0)
        elif op == "delete":
            assert i1 > i0
            f._lines_deleted.update(range(i0, i1))
        elif op == "replace":
            assert i1 > i0
            f._lines_replaced.update(range(i0, i1))
        elif op == "equal":
            pass
        else:
            assert 0, "unknown op %s" % op
    lines_diff = ops_stochastic_expand(lines_diff,
        left_prob=1, right_prob=1,
        exact_cx_lines0=exact_cx_lines0, exact_cx_lines1=exact_cx_lines1,
        disable_insert=True   # we don't like pure inserts, because without deleted lines the position to delete is only defined by the line number, therefore model arithmetic
    )
    lines_diff = ops_remove_short_equals(lines_diff, upto=2)
    for op, i0, i1, j0, j1 in lines_diff:
        if op == "equal":
            continue
        assert op in ["replace", "joined", "insert", "delete"], op
        c = ChunkElement(f)
        c.assign_from_diff(dest_text[j0:j1], i0, i1, j0, j1)
        chunks.append(c)
        f.add_expanding_range(line0=i0, line1=i1-1, aux=0)
    return chunks
