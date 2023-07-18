from .gpt_toolbox_spad import ScratchpadToolboxGPT
from typing import Dict, List
from .utils import msg


class GptExplainCodeBlock(ScratchpadToolboxGPT):
    def _messages(self) -> List[Dict[str, str]]:
        _, _, ctxt = self.trim_context()
        return [
            *explain_code_block_ctxt(),
            msg('user', ctxt),
            msg('assistant',
                "Thanks for giving me the context. "
                "I understand it. "
                "Please provide me the part of code you need to explain in a block."
                ),
            msg('user', self.selection)
        ]

    def _postprocess(self, completion: str) -> str:
        completion = code_block_postprocess(completion)
        return self._txt[:self.cursor1] + '\n' + completion + self._txt[self.cursor1:]



class GptExplainCodeBlockGPT4(GptExplainCodeBlock):
    def __init__(self, **kwargs):
        super().__init__(
            model_n='gpt-4',
            **kwargs
        )
