from typing import Dict, List

from refact_scratchpads_no_gpu.gpt_toolbox.gpt_utils import msg
from refact_scratchpads_no_gpu.gpt_toolbox.gpt_toolbox_spad import ScratchpadToolboxGPT


class GptCompletion(ScratchpadToolboxGPT):
    def _messages(self) -> List[Dict[str, str]]:
        cursor0, _, ctxt = self.trim_context()
        ctxt = ctxt[:cursor0] + '<|complete-me|>' + ctxt[cursor0:]
        return [
            msg(
                'system',
                "You are an AI programming assistant.\n"
                "Follow the user's requirements carefully and to the letter."
            ),
            msg(
                'user',
                ctxt
            ),
            msg(
                'assistant',
                'What do I need to do with this code?'
            ),
            msg(
                'user',
                "Replace <|complete-me|> with the code completion. "
                "Write it in the block of code. "
                "Do not explain anything. "
                "Write only the code completion."
            )
        ]


class GptCompletionGPT4(GptCompletion):
    def __init__(self, **kwargs):
        super().__init__(
            model_n='gpt-4',
            **kwargs
        )

    def _postprocess(self, completion: str) -> str:
        # Output of GPT-4 does not need to be postprocessed, such as find ```
        return self._txt[:self.cursor0] + completion + self._txt[self.cursor1:]

