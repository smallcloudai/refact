import termcolor

from itertools import groupby
from typing import List, Iterable, Tuple

from refact_encoding.encoding import RefactEncoding


__all__ = ["hlprint", "editclass_print"]


def _colored_join(sequence: Iterable[Tuple[str, str, str]]):
    keyfunc = lambda item: item[1:3]
    return "".join([
        termcolor.colored("".join([text for text, _, _ in group]),
                          color=color, on_color=on_color)
        for (color, on_color), group in groupby(sequence, keyfunc)
    ])


def hlprint(encoder: RefactEncoding, tokens: List[int], mask1=None, mask2=None) -> str:

    def _decode_colored_g():
        for idx, token in enumerate(tokens):
            text = encoder.decode([token])
            color = None
            on_color = None
            if mask1 is not None and mask1[idx]:
                if token == encoder.ESCAPE:
                    on_color = "on_green"
                else:
                    color = "green"
            elif mask2 is not None and mask2[idx]:
                if token == encoder.ESCAPE:
                    on_color = "on_magenta"
                else:
                    color = "magenta"
            elif token == encoder.DIAMOND:
                color, on_color = "red", "on_white"
            elif token in [encoder.ESCAPE, encoder.INFILL, encoder.MSG, encoder.FILE, encoder.CHUNK]:
                color, on_color = "red", "on_white"
            elif token == encoder.EOT or encoder.is_tpos(token):
                color = "red"
            yield text, color, on_color

    return _colored_join(_decode_colored_g())


# TODO: typing, unclear diffedits format
def editclass_print(encoder: RefactEncoding, tokens: List[int], mask, diffedits) -> str:

    def _decode_colored_g():
        for token, m, diffedit in zip(tokens, mask, diffedits):
            text = encoder.decode([token])
            color = None
            on_color = None
            if diffedit == 1:  # no edit
                on_color = "on_blue"
            elif diffedit == 2:  # edit
                if token == encoder.LF:
                    color, text = "yellow", "EDIT\n"
                else:
                    color = "red"
            elif diffedit == 3:  # continue
                if token == encoder.LF:
                    color, text = "yellow", "MOAR\n"
                else:
                    color = "magenta"
            elif m:
                if token == encoder.ESCAPE:
                    on_color = "on_green"
                else:
                    color = "green"
            elif token == encoder.DIAMOND:
                color, on_color = "grey", "on_white"
            elif token in [encoder.ESCAPE, encoder.INFILL, encoder.MSG, encoder.FILE, encoder.CHUNK]:
                color, on_color = "grey", "on_white"
            else:
                color = "blue"
            yield text, color, on_color

    return _colored_join(_decode_colored_g())
