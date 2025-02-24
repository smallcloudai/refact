import random
from code_contrast.format_2023q2.element import Element, ElementPackingContext, ElementUnpackContext
from dataclasses import dataclass, field
from typing import List, Tuple, Optional, Set


@dataclass
class _FileExpandingRange:
    line0: int
    line1: int
    aux: int
    line0expand: int = -1
    line1expand: int = -1
    works0: int = 1
    works1: int = 1


class FileElement(Element):
    def __init__(self, file_fn: str, file_lines: List[str]):
        super().__init__("FILE")
        self.file_fn = file_fn
        self.file_lines = file_lines
        self.file_lines_toks: List[Optional[List[int]]] = []
        self._footer_toks = list()
        self._lineheaders_dirty = True
        self._lineheaders_cnt_n = 0
        self._lineheaders_aux_n = 0
        self._toks_count_LINE = -1
        self._expanding_ranges: List[_FileExpandingRange] = list()
        self._cursor_token_at_line = -1
        self._lines_inspoints: Set[int] = set()
        self._lines_deleted: Set[int] = set()
        self._lines_replaced: Set[int] = set()
        self._file_lookup_helper_string: str = ""    # All lines converted to tokens, joined back into a string

    def add_expanding_range(self, line0: int, line1: int, aux: int):
        self._expanding_ranges.append(_FileExpandingRange(
            line0=max(0, min(line0, len(self.file_lines) - 1)),
            line1=max(0, min(line1, len(self.file_lines) - 1)),
            aux=aux))

    def pack_init(self, cx: ElementPackingContext) -> Tuple[List[int], List[int]]:
        header_toks = cx.enc.encode("FILE " + self.file_fn.replace("\n", "\\n") + "\n")
        self._toks_count_LINE = len([cx.enc.ESCAPE] + cx.enc.encode("LINE%04d\n" % 1234))
        self._footer_toks = [cx.enc.ESCAPE] + cx.enc.encode("/FILE\n")
        cx.filled_aux_n += len(self._footer_toks)
        t, m = [], []
        t.extend(header_toks)
        m.extend([1]*len(header_toks))
        # Each range has a header, until it bumps into another range above when exanding
        self.file_lines_toks = [None] * len(self.file_lines)
        self._lineheaders_dirty = True
        self._lineheaders_cnt_n = 0
        self._lineheaders_aux_n = 0
        for er in self._expanding_ranges:
            er.works0 = 1 if not cx.for_training else random.randint(0, 50)
            er.works1 = 1 if not cx.for_training else random.randint(0, 50)
            er.line0expand = er.line0
            er.line1expand = er.line1
            for line in range(er.line0expand, er.line1expand + 1):
                self._lines2toks_helper(cx, line, aux=er.aux, mandatory=True)
        self._estimate_line_header_tokens(cx)
        return t, m

    def _estimate_line_header_tokens(self, cx: ElementPackingContext):
        if not self._lineheaders_dirty:
            return
        # Intersecting ranges will make the estimation larger than it should be, causing this
        # calculation to be more conservative => the end result is a less filled context.
        cnt_lineheaders_n = sum(
            1 + (er.line1expand - er.line0expand + 1) // cx.fmt.LINE_NUMBER_EACH
            for er in self._expanding_ranges if not er.aux
        )
        aux_lineheaders_n = sum(
            1 + (er.line1expand - er.line0expand + 1) // cx.fmt.LINE_NUMBER_EACH
            for er in self._expanding_ranges if er.aux
        )
        self._lineheaders_dirty = False
        if cnt_lineheaders_n != self._lineheaders_cnt_n:
            cx.filled_ctx_n += (cnt_lineheaders_n - self._lineheaders_cnt_n) * self._toks_count_LINE
            self._lineheaders_cnt_n = cnt_lineheaders_n
        if aux_lineheaders_n != self._lineheaders_aux_n:
            cx.filled_aux_n += (aux_lineheaders_n - self._lineheaders_aux_n) * self._toks_count_LINE
            self._lineheaders_aux_n = aux_lineheaders_n

    def _lines2toks_helper(self, cx: ElementPackingContext, l: int, aux: int, mandatory: bool) -> bool:
        self._estimate_line_header_tokens(cx)
        if l < 0 or l >= len(self.file_lines):
            return False
        if self.file_lines_toks[l] is not None:
            return False
        t = cx.enc.encode(self.file_lines[l])
        len_t = len(t)
        if self._cursor_token_at_line == l:
            len_t += 2
        take_line = False
        if aux:
            if cx.filled_aux_n + len_t < cx.limit_aux_n or mandatory or cx.for_training:
                # print("take aux line %i" % (l))
                cx.filled_aux_n += len_t
                take_line = True
        else:
            if cx.filled_ctx_n + len_t < cx.limit_ctx_n + (cx.limit_aux_n - cx.filled_aux_n) or mandatory or cx.for_training:
                # print("take ctx line %i" % (l))
                cx.filled_ctx_n += len_t
                take_line = True
        if not take_line:
            return False
        self.file_lines_toks[l] = t
        self._lineheaders_dirty = True
        return True

    def pack_inflate(self, cx: ElementPackingContext, aux: bool) -> bool:
        if cx.for_training:
            aux = False
        anything_works = False
        for ri, er in enumerate(self._expanding_ranges):
            if er.aux != aux:
                continue
            if er.works0:
                # if er.line0expand - 1 > 0 and self.file_lines_toks[er.line0expand - 1] is not None:
                #     print(" ! bumped into another expanding range er.line0expand - 1 = %d" % (er.line0expand - 1))
                #     er.works0 = 0
                success = self._lines2toks_helper(cx, er.line0expand - 1, aux=er.aux, mandatory=False)
                if success:
                    er.line0expand -= 1
                    if cx.for_training:
                        er.works0 -= 1    # Works as a counter up to a random number
                else:
                    er.works0 = 0
            if er.works1:
                # For example we start with the range (5, 5) and expand from there, the line below is 6
                success = self._lines2toks_helper(cx, er.line1expand + 1, aux=er.aux, mandatory=False)
                if success and cx.for_training:
                    er.works1 -= 1
                if success and er.line1expand + 1 >= len(self.file_lines) - 1:
                    er.works1 = 0
                    er.line1expand = len(self.file_lines) - 1
                elif success:
                    er.line1expand += 1
                    assert er.line1expand < len(self.file_lines), ri
                else:
                    er.works1 = 0
            # print("range%d: %d..%d, %d, %d, aux=%d, need_header=%i" % (ri, er.line0expand, er.line1expand, er.works0, er.works1, er.aux, er.need_header))
            anything_works |= er.works0 or er.works1
        return anything_works

    def pack_finish(self, cx: ElementPackingContext) -> Tuple[List[int], List[int]]:
        t, m = [], []
        assert len(self.file_lines) == len(self.file_lines_toks)
        self._file_lookup_helper_string = ""
        line_countdown = 0
        for line_n, line_toks in enumerate(self.file_lines_toks):
            if not line_toks:
                line_countdown = 0
                self._file_lookup_helper_string += "\n"
                continue
            if line_countdown == 0:
                line_n_t = [cx.enc.ESCAPE] + cx.enc.encode("LINE%04d\n" % (line_n,))
                t.extend(line_n_t)
                m.extend([0]*len(line_n_t))
                line_countdown = 15
            if self._cursor_token_at_line == line_n:
                t.extend([cx.enc.ESCAPE, cx.enc.CURSOR])
                m.extend([0, 0])
            t.extend(line_toks)
            m.extend([1]*len(line_toks))
            self._file_lookup_helper_string += self.file_lines[line_n]
            line_countdown -= 1
        t.extend(self._footer_toks)
        m.extend([1]*len(self._footer_toks))
        return t, m

    @classmethod
    def unpack_init(cls, cx: ElementUnpackContext, init_tokens: List[int]) -> Element:
        raise ValueError("Unpacking is not supported for FILE, because most likely it's not a complete file in the context. Reuse the file used to generate the context.")
