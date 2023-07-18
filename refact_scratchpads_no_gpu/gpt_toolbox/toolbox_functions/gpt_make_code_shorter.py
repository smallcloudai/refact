from typing import Dict, List

from refact_scratchpads_no_gpu.gpt_toolbox.gpt_utils import msg
from refact_scratchpads_no_gpu.gpt_toolbox.gpt_toolbox_spad import ScratchpadToolboxGPT


class GptMakeCodeShorter(ScratchpadToolboxGPT):
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
                "I will provide you the code that is suboptimal, verbose and complicated. "
                "You need to replace the suboptimal code with a shorter and more simple code. "
                "The code you generated will be placed in the context file, "
                "so keep all styles and indents. "
                "Do not explain anything. "
                "Firstly, I will provide you the whole file -- the context. "
                "Then you will receive a piece of code you will simplify. "
                "When the block of code ends you will print <|end|> after it."
            ),
            msg(
                'assistant',
                "Provide me a context."
            ),
            msg('user',
                """
class Person:
    def __init__(self, name, age):
        self.name = name
        self.age = age


class People:
    def __init__(people):
        self.people = []
        for p in people:
            name = p[0]
            age = p[1]
            person = Person(name, age)
            self.people.append(person)

    def __iter__(self):
        yield from self.people

            """),
        msg('assistant',
            "Please provide me the code you need to simplify."
            ),
        msg('user',
            """
        self.people = []
        for p in people:
            name = p[0]
            age = p[1]
            person = Person(name, age)
            self.people.append(person)
            """),
        msg('assistant',
            """
```
        self.people = [Person(name, age) for name, age in people]
```
<|end|>"""),
            msg('user', ctxt),
            msg('assistant',
                "Thanks for giving me the context. "
                "Please provide me the part of code you need to simplify."
                ),
            msg('user', self.selection)
        ]


class GptMakeCodeShorterGPT4(GptMakeCodeShorter):
    def __init__(self, **kwargs):
        super().__init__(
            model_n='gpt-4',
            **kwargs
        )
