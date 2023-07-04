from code_contrast.format_2023q2.element import Element, ElementPackingContext, ElementUnpackContext
from typing import List, Tuple


class MsgElement(Element):
    def __init__(self, msg_role: str, msg_text: str):
        super().__init__("MSG")
        self.msg_role = msg_role
        self.msg_text = msg_text
        self._unpack_tokens: List[int] = []


    def pack_init(self, cx: ElementPackingContext) -> Tuple[List[int], List[int]]:
        toks = cx.enc.encode(self.msg_role + " " + self.msg_text + "\n")
        return toks, [1]*len(toks)


    @classmethod
    def unpack_init(cls, cx: ElementUnpackContext, init_tokens: List[int]) -> Element:
        t0 = init_tokens[0]
        if t0 == cx.enc.ESCAPE:
            init_txt = cx.enc.decode(init_tokens[1:])
        else:
            assert 0, "Cannot parse msg %s" % cx.enc.decode(init_tokens)
        return MsgElement(init_txt, "")

    def unpack_more_tokens(self, cx: ElementUnpackContext) -> bool:
        while len(cx.tokens):
            t = cx.tokens[0]
            if cx.fmt.is_special_token(t):
                return True
            self._unpack_tokens.append(cx.tokens.pop(0))
        return False

    def unpack_finish(self, cx: ElementUnpackContext):
        t = cx.enc.decode(self._unpack_tokens)
        if t.startswith(" "):
            t = t[1:]
        if t.endswith("\n"):
            t = t[:-1]
        self.msg_text = t
