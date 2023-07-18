from typing import Dict


def msg(role: str, content: str) -> Dict[str, str]:
    assert role in ['system', 'user', 'assistant']
    return {'role': role, 'content': content}


def completion_ctxt():
    return [
        msg(
            'system',
            "You are an AI programming assistant.\n"
            "Follow the user's requirements carefully & to the letter."
        ),
        msg(
            'user',
            "I will provide you a code and then follow up my instructions to the letter."
        ),
    ]



def explain_code_block():
    return [
        msg(
            'system',
            'You are a professional high-class code assistant. '
            'Your were created to modify, generate, interpret and generalize code'
        ),
        msg(
            'user',
            'I will provide you a piece of code, you to provide an explanation what the code is doing. '
            'You must explain it briefly. Split explanations into multiple lines to make it easier to read in IDE (up to 50 symbols per line). '
            'Wrap explanations in comment blocks. '
            'Is it clear?'
        ),
        msg(
            'assistant',
            "Yes, it's clear. Please provide the code you want me to explain briefly."
        ),
        msg(
            'user',
            """
    def completion(self, final: bool, tokens_batch: Optional[int] = 25) -> Iterator[Dict[str, str]]:
        tokens_batch: int = self.max_tokens if final else tokens_batch

        return self.completion_stream(
            engine=self._engine,
            tokens_batch=tokens_batch,
            prompt=self.prompt,
            replace_modified=self._replace_modified
        )
            """),
        msg(
            'assistant',
            """
```
# This code defines a function called "completion"
# which takes two arguments: final which is boolean and tokens_batch, which is optional int
# if final = true, streaming is disabled, and vice versa
# tokens batch will be set to self.max_tokens if streaming is disabled
# it returns a function self._completion_stream, which receives engine, tokens_batch, prompt and replace_modified
# this function returns an iterator of Dicts, where keys and values are both strings
```
            """),
        msg(
            'user',
            'Thanks for your help! That is 100% precise for what I meant.'
        ),
        msg(
            'assistant',
            'Anytime! Give me another piece of code you need to explain in the same way. I am always ready to help you.'
        )
    ]


def add_console_logs():
    return [
        msg(
            'system',
            'You are a professional high-class code assistant. '
            'Your were created to modify, generate, interpret and generalize code'
        ),
        msg(
            'user',
            'I will provide you a piece of code, you need to add console logs. '
            'You have to localize places in a given piece of code where console logs might be useful for debugging '
        ),
        msg(
            'user',
            """
    def __call__(self, filename: str) -> Optional[str]:
        for filt in self._filters:
            filtered = filt([filename])
            if filtered:
                return filt.language
        return None
            """),
        msg(
            'assistant',
            """
```
    def __call__(self, filename: str) -> Optional[str]:
        print(f'filename: {filename}')
        print(f'filters: {self._filters}')
        for it, filt in enumerate(self._filters):
            print(f'it: {it}')
            print(f'filt: {filt}')
            filtered = filt([filename])
            print(f'filtered: {filtered}')'
            if filtered:
                print(f'language: {filt.language}')
                return filt.language
        print('language: None')
        return None
```
            """),
    ]


def precise_naming_ctxt():
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
        msg('assistant',
            """
```
def polynom(x, coefficients):
    value = 0
    x_power = 1
    for c in coefficients:
        value += x_power * c
        x_power *= x
    return value
```
<|end|>
            """),
    ]

