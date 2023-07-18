from .gpt_toolbox_spad import ScratchpadToolboxGPT
from typing import Dict, List
from .utils import msg, code_block_postprocess


class GptExplainCodeBlock(ScratchpadToolboxGPT):
    def _messages(self) -> List[Dict[str, str]]:
        _, _, ctxt = self.trim_context()
        return [
            msg(
                'system',
                "You are an AI programming assistant.\n"
                "Follow the user's requirements carefully & to the letter."
            ),
            msg(
                'user',
                "I will provide you the code that I do not understand and I need you to explain me it."
                "I need you understand the logic of this code and t"
                "hen output the block of code that contain comments that explain the given code. "
                "Do not explain anything outside of a block of code. "
                "Firstly, I will provide you the context, which is the whole file. "
                "Then I will provide you the specific piece of code you will explain in a block. "
                "When the block of code ends you will print <|end|> after it."
            ),
            msg(
                'assistant',
                "Provide me a context."
            ),
            msg('user',
                """
from typing import *

from .chatgpt_scratchpad import ScratchpadToolboxGPT


class ScratchpadMakeCodeShorter(ScratchpadToolboxGPT):
    def __init__(self, **kwargs):
        super().__init__(**kwargs)

    def _messages(self):
        return [*make_code_shorter(), msg('user', self._selection)]

    def completion(self, final, tokens_batch=25):
        tokens_batch: int = self.max_tokens if final else tokens_batch

        return self.completion_stream(
            engine=self._engine,
            tokens_batch=tokens_batch,
            prompt=self.prompt,
            replace_modified=self._replace_modified
        )

    def _postprocess(self, completion):
        return self._txt[:self.cursor0] + completion + self._txt[self.cursor1:]
"""),
            msg('assistant',
                "Provide me the code you need to explain in a block."
                ),
            msg('user',
                """
    def completion(self, final, tokens_batch=25):
        tokens_batch: int = self.max_tokens if final else tokens_batch

        return self.completion_stream(
            engine=self._engine,
            tokens_batch=tokens_batch,
            prompt=self.prompt,
            replace_modified=self._replace_modified
        )
"""),
            msg('assistant',
                """
```
# This code defines a function called "completion"
# which takes two arguments: final which is boolean and tokens_batch, which is optional int
# if final = true, streaming is disabled, and vice versa
# tokens batch will be set to self.max_tokens if streaming is disabled
# it returns a function self._completion_stream, which receives engine, tokens_batch, prompt and replace_modified
# this function returns an iterator of Dicts, where keys and values are both strings
```
<|end|>"""),
            msg('user', ctxt),
            msg('assistant',
                "Thanks for giving me the context. "
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
