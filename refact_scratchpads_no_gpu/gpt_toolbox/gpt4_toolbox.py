from typing import *

import ujson as json

from .gpt_toolbox_spad import ScratchpadToolboxGPT

from .gpt35_prompts import msg
from .gpt4_prompts import code_review

from .utils import find_substring_positions
from .gpt35_toolbox import \
    ScratchpadExplainCodeBlock, \
    ScratchpadCompletion, ScratchpadDetectBugsHighlight


class ScratchpadExplainCodeBlockGPT4(ScratchpadExplainCodeBlock):
    def __init__(self, **kwargs):
        super().__init__(
            model_n='gpt-4',
            **kwargs
        )


class ScratchpadCompletionGPT4(ScratchpadCompletion):
    def __init__(self, **kwargs):
        super().__init__(
            model_n='gpt-4',
            **kwargs
        )

    def _postprocess(self, completion: str) -> str:
        return self._txt[:self.cursor0] + completion + self._txt[self.cursor1:]


