import re
from code_contrast.format_2023q2.element import Element, ElementPackingContext, ElementUnpackContext
from code_contrast.format_2023q2.el_file import FileElement
from typing import List, Tuple, Optional, Dict


STATE_DEL, STATE_LINE_N, STATE_INS = "DEL", "LINE_N", "INS"


class ChunkElement(Element):
    def __init__(self, orig_file: Optional[FileElement]):
        super().__init__("CHUNK")
        self.orig_file = orig_file
        self.line_n = -1
        self.to_del: List[str] = []
        self.to_ins: List[str] = []
        self.fuzzy = -1
        self.error = ""
        self._decode_state = STATE_DEL
        self._ins_tokens: List[int] = []
        self._del_tokens: List[int] = []
        self._tok_LINE = -1
        self._hint_line = -1
        self._hint_file = ""
        self._line_tokens: List[int] = []
        self._line_n_patched = -1

    def assign_from_diff(self, replacement_text: List[str], i0, i1, j0, j1):
        assert self.orig_file
        self.line_n = i0
        self.to_del = self.orig_file.file_lines[i0:i1]
        self.to_ins = replacement_text
        self.fuzzy = 0

    def pack_init(self, cx: ElementPackingContext) -> Tuple[List[int], List[int]]:
        assert self.orig_file
        t = cx.enc.encode("CHUNK\n")
        for j in range(len(self.to_del)):
            t.extend(cx.enc.encode(self.to_del[j]))
        t.extend([cx.enc.ESCAPE] + cx.enc.encode("LINE%04d %s\n" % (self.line_n, self.orig_file.file_fn)))
        for j in range(len(self.to_ins)):
            t.extend(cx.enc.encode(self.to_ins[j]))
        m = [1]*len(t)
        return t, m


    @classmethod
    def unpack_init(cls, cx: ElementUnpackContext, init_tokens: List[int]) -> Element:
        el = ChunkElement(None)
        def should_be_single_token(s):
            seq = cx.enc.encode(s)
            assert len(seq) == 1, "\"%s\" is not one token %s, first token is \"%s\"" % (s, seq, cx.enc.decode([seq[0]]).replace("\n", "\\n"))
            return seq[0]
        el._tok_LINE = should_be_single_token("LINE")
        el._state = STATE_DEL
        return el

    def _switch_state(self, cx, new_state):
        # print(" -- switch state %s -> %s" % (self._state, new_state))
        if self._state == STATE_LINE_N:
            tmp = cx.enc.decode(self._line_tokens)
            try:
                # Format is "0008 test.py", filename can contain spaces, slashes, etc
                m = re.fullmatch(r"^(\d+) (.+)\n.*", tmp)
                if m:
                    self._hint_line = int(m.group(1))
                    self._hint_file = m.group(2)
            except ValueError:
                pass   # stays -1
            # print("LINE collected self._line_tokens \"%s\" -> _hint_line %i _hint_file '%s'" % (tmp.replace("\n", "\\n"), self._hint_line, self._hint_file))
            self._line_tokens = []
            # fills fuzzy correctly, even if we know the location already
            self._locate_this_chunk_in_file_above(cx, force=True)
        self._state = new_state

    def unpack_more_tokens(self, cx: ElementUnpackContext) -> bool:
        while len(cx.tokens) > 1:
            t0 = cx.tokens[0]
            t1 = cx.tokens[1]
            # print("chunk.unpack %5i \"%s\"" % (t0, cx.enc.decode([t0]).replace("\n", "\\n")))
            if cx.fmt.is_special_token(t0):
                if self._state == STATE_DEL and t1 == self._tok_LINE:
                    self._switch_state(cx, STATE_LINE_N)
                    del cx.tokens[:2]
                    continue
                else:
                    # print("special token, must be next element, chunk over")
                    return True
            if self._state == STATE_LINE_N:
                t1_txt = cx.enc.decode([t1])
                self._line_tokens.append(t0)
                if "\n" in t1_txt:
                    self._line_tokens.append(t1)   # We're hedging here: maybe this token contributes to line, maybe to code, maybe both!
                    self._switch_state(cx, STATE_INS)
                del cx.tokens[0]
            elif self._state == STATE_INS:
                self._ins_tokens.append(cx.tokens.pop(0))
            elif self._state == STATE_DEL:
                self._del_tokens.append(cx.tokens.pop(0))
                self._locate_this_chunk_in_file_above(cx, force=False)
            else:
                assert 0, "unknown state %s" % self._state
        return False

    def unpack_finish(self, cx: ElementUnpackContext):
        to_del_str = self._del_str(cx)
        to_ins_str = self._ins_str(cx)
        self.to_del = to_del_str.splitlines(keepends=True)
        self.to_ins = to_ins_str.splitlines(keepends=True)

    def _del_str(self, cx):
        if len(self._del_tokens):
            to_del_str = cx.enc.decode(self._del_tokens)
            if not to_del_str.startswith("\n"):
                raise ValueError("there is no \\n in between CHUNK and deleted text")
        else:
            to_del_str = "\n"
        return to_del_str[1:]

    def _ins_str(self, cx):
        if len(self._ins_tokens):
            to_ins_str = cx.enc.decode(self._ins_tokens)
            if not to_ins_str.startswith("\n"):
                raise ValueError("there is no \\n in between LINE and inserted text")
        else:
            to_ins_str = "\n"
        return to_ins_str[1:]

    def _locate_this_chunk_in_file_above(self, cx: ElementUnpackContext, force: bool):
        if not self.orig_file or force:
            lst: List[Tuple[FileElement, int, int]] = []
            to_del_str = self._del_str(cx)
            # possible locations
            lst = cx.lookup_file(to_del_str, self._hint_line, self._hint_file, up_to_matches=(5 if not force else -1))
            if len(lst) == 1:
                pass
            elif force and len(lst) > 1:
                print("WARNING: multiple matches %i for todel, using first one, lookup was hint_line=%i, hint_file=\"%s\"" % (len(lst), self._hint_line, self._hint_file.replace("\n", "\\n")))
                print("\n".join([str(x) for x in lst]))
            elif force:
                print("WARNING: no matches for todel=\"%s\", lookup was hint_line=%i, hint_file=\"%s\"" % (to_del_str[:100].replace("\n", "\\n"), self._hint_line, self._hint_file.replace("\n", "\\n")))
                # to_del = to_del_str.splitlines(keepends=True)
                # for i in range(len(to_del)):
                #     eq = self.orig_file.file_lines[self.line_n + i] == to_del[i]
                #     have = self.orig_file.file_lines_toks[self.line_n + i] is not None
                #     print("line %i eq=%i have=%i" % (self.line_n + i, eq, have))
                return
            else:
                # nothing found, but not force yet, so not a big deal
                return
            # print("xxx\n" + "\n".join([str(x) for x in lst]))
            self.orig_file, self.line_n, self.fuzzy = lst[0]
            if force and self.fuzzy != 0:
                print("WARNING: fuzzy not zero chunk, todel=\"%s\" hints line_n=%i, line=%s" % (to_del_str[:100].replace("\n", "\\n"), self.line_n, self.orig_file.file_lines[self.line_n]))


def apply_chunks(plan: List[Element]) -> Dict[str, List[str]]:
    code: Dict[str, List[str]] = {}
    for el in plan:
        if isinstance(ch := el, ChunkElement):
            ch._line_n_patched = ch.line_n
    ch: ChunkElement
    for plan_i, ch in enumerate(plan):
        if not isinstance(ch, ChunkElement):
            continue
        # print("applying chunk plan=%i" % plan_i)
        fn = ch.orig_file.file_fn
        if fn not in code:
            code[fn] = ch.orig_file.file_lines[:]
        gets_deleted = code[fn][ch._line_n_patched : ch._line_n_patched + len(ch.to_del)]
        if gets_deleted != ch.to_del:
            import IPython; IPython.embed(); quit()
        assert gets_deleted == ch.to_del, "Oops sanity check failed.\n----------existing code1:\n%s\n----------existing code2:\n%s" % ("".join(gets_deleted), "".join(ch.to_del))
        code[fn][ch._line_n_patched : ch._line_n_patched + len(ch.to_del)] = ch.to_ins
        forward_ch: ChunkElement
        for forward_ch in plan[plan_i+1:]:
            if not isinstance(forward_ch, ChunkElement):
                continue
            if forward_ch.orig_file != ch.orig_file:
                continue
            if forward_ch._line_n_patched < ch._line_n_patched:
                continue
            old_line_n = forward_ch._line_n_patched
            forward_ch._line_n_patched += (len(ch.to_ins) - len(ch.to_del))
            # print("xxx modifying forward chunk line_n %i -> %i" % (old_line_n, forward_ch._line_n_patched))
    return code


