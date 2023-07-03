import termcolor
from code_contrast.encoding.smc_encoding import SMCEncoding
from typing import List, Dict, Tuple, Callable, Type

from code_contrast.contrast_2023q2.element import Format2023q2, Element, ElementPackingContext
from code_contrast.contrast_2023q2.el_msg import MsgElement
from code_contrast.contrast_2023q2.el_chunk import ChunkElement



def format_2023q2_escape(enc: SMCEncoding) -> Format2023q2:
    fmt = Format2023q2(enc)
    fmt.element_start_seq = {
    "SYSTEM": [enc.ESCAPE, *enc.encode("SYSTEM")],
    "USER": [enc.ESCAPE, *enc.encode("USER")],
    "ASSISTANT": [enc.ESCAPE, *enc.encode("ASSISTANT")],
    "CHUNK": [enc.ESCAPE, *enc.encode("CHUNK")],
    }
    fmt.element_classes = {
    "SYSTEM": MsgElement,
    "USER": MsgElement,
    "ASSISTANT": MsgElement,
    "CHUNK": ChunkElement,
    }
    ESCAPE = enc.ESCAPE
    fmt.is_special_token = lambda t: t==ESCAPE
    return fmt

