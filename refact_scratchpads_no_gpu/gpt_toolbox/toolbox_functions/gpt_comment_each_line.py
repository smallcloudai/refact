from typing import Dict, List

from refact_scratchpads_no_gpu.gpt_toolbox.gpt_utils import msg
from refact_scratchpads_no_gpu.gpt_toolbox.gpt_toolbox_spad import ScratchpadToolboxGPT


class GptCommentEachLine(ScratchpadToolboxGPT):
    def _messages(self) -> List[Dict[str, str]]:
        return [
            msg(
                'system',
                'You are a professional high-class code assistant. '
                'Your were created to modify, generate, interpret and generalize code'
            ),
            msg(
                'user',
                "I have an assignment for you, so listen attentively. "
                "I will provide you a piece of code, you need to explain each line of this piece of code. "
                "I need to know what each line of the piece of code does. "
                "You need to return the exact same piece of code given, but each line must have an explanation in comment above it"
                "Do you understand your assignment?"
            ),
            msg(
                'assistant',
                "Yes, my assignment is clear. "
                "I am receiving a piece of code and I need to explain each line of it. "
                "I have to return the exact same piece of code, but each line must have an explanation in comment above it. "
                "I'm willing to do it for you! However, I still have some questions:\n"
                "1. If the code already has some comments, should I keep them or remove?\n"
                "2. If the code has any other issues that I can detect, may I fix them?"
            ),
            msg(
                'user',
                "If you see existing comments in the piece of code, you must keep them, you should ignore them, no explaination for them needed. "
                "If you see some other issues in the code, you are prohibited react on them in any way, don't fix them. Ignore them! "
                "When code block ends you must print <|end|> after it. "
                "Is it clear?"
            ),
            msg(
                'assistant',
                "Yes, absolutely! "
                "If I see existing comments in provided piece of code, I must keep them and I must not explain them.\n"
                "If I see any other issues in the piece of code, I will not try to fix them. It is prohibited.\n"
                "When code block ends I will print <|end|> after it.\n"
                "I have no doubts I can manage this task! Please provide me a piece of code each line of which I need explain."
            ),
            msg('user',
                """
    def completion(self, final: bool, tokens_batch: Optional[int] = 25) -> Iterator[Dict[str, str]]:
        tokens_batch: int = self.max_tokens if final else tokens_batch

        # implement more cool features
        return self.completion_stream(
            # engine must be one of the one in docs
            engine=self._engine,
            tokens_batch=tokens_batch,
            prompt=self.prompt,
            replace_modified=self._replace_modified
        )
"""),
            msg('assistant',
                """
```
    def completion(self, final: bool, tokens_batch: Optional[int] = 25) -> Iterator[Dict[str, str]]:
        # if not tokens_batch given, using max_tokens
        tokens_batch: int = self.max_tokens if final else tokens_batch

        # implement more cool features
        return self.completion_stream(
            # engine is a model codify API uses. E.g.  text-davinci-003, code-davinci-002 etc
            # engine must be one of the one in docs
            engine=self._engine,
            # how many tokens will be in each batch
            tokens_batch=tokens_batch,
            # function that returns prompt for selected engine
            prompt=self.prompt,
            # replace selection from original code with generated code
            replace_modified=self._replace_modified
        )
```
<|end|>"""),
            msg(
                'user',
                'Thanks for your help! That is 100% precise for what I meant.'
            ),
            msg(
                'assistant',
                'Anytime! Give me another piece of code you need to explain each line. It is a pleasure to help you!'
            ),
            msg('user', self.selection)
        ]
