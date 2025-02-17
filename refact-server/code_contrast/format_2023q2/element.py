import termcolor, re
from typing import List, Dict, Tuple, Callable, Type


class Format2023q2:
    def __init__(self, enc):
        self.enc = enc
        self.element_start_seq: Dict[str, List[int]] = {}
        self.element_classes: Dict[str, Type[Element]] = {}
        self.is_special_token = lambda t: False
        self.LINE_NUMBER_EACH = 15


class ElementPackingContext:
    def __init__(self, fmt: Format2023q2, limit_ctx_n: int, limit_aux_n: int, for_training: bool):
        self.fmt = fmt
        self.enc = fmt.enc
        self.for_training = for_training
        self.limit_ctx_n = limit_ctx_n
        self.limit_aux_n = limit_aux_n
        self.filled_ctx_n = 0
        self.filled_aux_n = 0
        self.occupied_line_ranges: List[Tuple[int, int]] = list()
        self.minimal_context_too_big_warning = False


class ElementUnpackContext:
    def __init__(self, fmt: Format2023q2, lookup_file: Callable):
        self.fmt = fmt
        self.enc = fmt.enc
        self.tokens: List[int] = list()
        self.lookup_file = lookup_file


class Element:
    def __init__(self, el_type: str):
        self.el_type = el_type
        self.located_at = -1


    # Pack: translate element into tokens
    def pack_init(self, cx: ElementPackingContext) -> Tuple[List[int], List[int]]:
        """
        Returns tokens and mask. Most elements produce its tokens using this one call, only a few
        can inflate to occupy more space.
        """
        raise NotImplementedError()

    def pack_inflate(self, cx: ElementPackingContext, aux: bool) -> bool:
        """
        Encode lines one by one, until limit_ctx_n / limit_aux_n is reached.
        Return True if there is more space to fill. The idea is to inflate
        elements round-robin.

        :param aux: count tokens against limit_aux_n
        """
        return False

    def pack_finish(self, cx: ElementPackingContext) -> Tuple[List[int], List[int]]:
        """
        Returns final tokens and mask, after inflation.
        """
        return [], []


    # Unpack: restore element from tokens
    @classmethod
    def unpack_init(cls, cx: ElementUnpackContext, init_tokens: List[int]) -> 'Element':
        raise NotImplementedError()

    def unpack_more_tokens(self, cx: ElementUnpackContext) -> bool:
        """
        This function must either:
         * Continously consume cx.tokens from the beginning, for example with cx.tokens.pop(0).
         * Wait until there is enough tokens for the complete element, del cx.tokes[0:N] to consume N at once.
        Or do both.
        Return False if more tokens are needed, True if the element cannot consume anymore, the unpacker
        should move to the next.
        """
        raise NotImplementedError()

    def unpack_finish(self, cx: ElementUnpackContext) -> None:
        """
        Called after unpack_more_tokens() returns True, or the model hits max_tokens and cannot
        produce more tokens.
        """
        pass


    def __repr__(self) -> str:
        ret = termcolor.colored(self.el_type, "white", attrs=["bold"]) + " "
        for field in self.__dict__:
            if field.startswith("_") or field == "el_type":
                continue
            val = getattr(self, field)
            if callable(val):
                continue
            val_str = repr(val)
            val_str = re.sub('\033\[.*?m', '', val_str)
            val_str = val_str[:40] + "... " if len(val_str) > 40 else val_str + " "
            ret += field + " " + termcolor.colored(val_str, "cyan") + " "
        return ret

