from typing import Dict, List

from refact_scratchpads_no_gpu.gpt_toolbox.gpt_utils import msg
from refact_scratchpads_no_gpu.gpt_toolbox.gpt_toolbox_spad import ScratchpadToolboxGPT


class GptPreciseNaming(ScratchpadToolboxGPT):
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
                "I will provide you the code that has ambitious, unclear and incorrect naming."
                "I need you understand the logic of this code and then output the block of code with fixed naming. "
                "Do not explain anything. "
                "Firstly, I will provide you the context, which is the whole file. "
                "Then I will provide you the specific piece of code you will fix naming in. "
                "When the block of code ends you will print <|end|> after it."
            ),
            msg(
                'assistant',
                "Provide me a context."
            ),
            msg('user',
                """
from typing import *


def f(x, c):
    r = 0
    xx = 1
    for cc in c:
        r += xx * cc
        xx *= x
    return r


def main():
    years = [1693, 1900, 2000]
    for year in years:
        if f(year):
            print(year, "is leap")
        else:
            print(year, "is not leap")

if __name__ == "__main__":
    main()
"""),
            msg('assistant',
                "Please provide me the code you need to fix naming in."
                ),
            msg('user',
                """
def f(x, c):
    r = 0
    xx = 1
    for cc in c:
        r += xx * cc
        xx *= x
    return r
"""),
            msg('assistant', """```
def polynom(x, coefficients):
    value = 0
    x_power = 1
    for c in coefficients:
        value += x_power * c
        x_power *= x
    return value
```<|end|>"""),
            msg('user', ctxt),
            msg('assistant',
                "Thanks for giving me the context. "
                "Please provide me the part of code you need to fix naming in."
                ),
            msg('user', self.selection)
        ]
