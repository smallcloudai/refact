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


def make_code_shorter_ctxt():
    return [
        msg(
            'system',
            "You are an AI programming assistant.\n"
            "Follow the user's requirements carefully & to the letter."
        ),
        msg(
            'user',
            "I will provide you the code that is suboptimal, verbose and complicated. "
            "You will read the code line by line and understand what each line does. "
            "Afterwards, you will replace the suboptimal code with short clear and simple code, "
            "keeping in mind the context. The code you generated will be placed in the context file, "
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
<|end|>
            """),
    ]


def fix_bug_ctxt():
    return [
        msg(
            'system',
            "You are an AI programming assistant.\n"
            "Follow the user's requirements carefully & to the letter."
        ),
        msg(
            'user',
            "I will provide you the code that contains bugs, which you will need to find and fix."
            "I need you understand the logic of this code and then output the block of rewritten debugged code. "
            "Do not explain anything! "
            "Firstly, I will provide you the context, which is the whole file. "
            "Then I will provide you the specific piece of code you will find bugs and fix them. "
            "When the block of code ends you will print <|end|> after it."
            "In case there are no bugs in the part of the code I need to find bugs in you will leave it as it is."
        ),
        msg(
            'assistant',
            "Provide me a context."
        ),
        msg('user',
            """
from utils import filter_filenames


class LanguagesFilter:
    def __init__(self, languages, percent_in_commit):
        self._percent_in_commit = percent_in_commit
        self._filters = [LanguageFilter(language) for language in languages]

    def __call__(self, filenames):
        return filter_filenames(filenames, self._filters, self._percent_in_commit)

class CodeLanguagesFilter:
    def __init__(self, languages):
        self._filters = [LanguageFilter(language) for language in languages]

    def __call__(self, filename):
        for (filt in self._filters):
            filtered = filt([filename)
            if (filtered):
                return filt.language
        return null

if __name__ == "__main__":
    pass
            """),
        msg('assistant',
            "Please provide me the code you need to fix bugs in. "
            ),
        msg('user',
            """
    def __call__(self, filename):
        for (filt in self._filters):
            filtered = filt([filename)
            if (filtered):
                return filt.language
        return null
    }
            """),
        msg('assistant',
            """
```
    def __call__(self, filename: str):
        for filt in self._filters:
            filtered = filt([filename])
            if filtered:
                return filt.language
        return None
```
<|end|>
            """),
    ]


def explain_code_block_ctxt():
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
<|end|>
            """),
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


def comment_each_line():
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
            "I have to return the exact same piece of code, but each line must have an explanation in comment above it"
            "I'm willing to do it for you! However, I still have some questions: \n"
            "1. If the code already has some comments, should I keep them or remove? \n"
            "2. If the code has any other issues that I can detect, may I fix them?"
        ),
        msg(
            'user',
            "If you see existing comments in the piece of code, you must keep them, you should never-mind them, no explain for them needed. "
            "If you see some other issues in the code, you are prohibited react on them, try to fix them and touch. Ignore them! "
            "When code block ends you must print <|end|> after it. "
            "Is it clear?"
        ),
        msg(
            'assistant',
            "Yes, absolutely! "
            "If I see existing comments in provided piece of code, I must keep them and I must not explain them. \n"
            "If I see any other issues in the piece of code, I will not try to fix them. It is prohibited. \n"
            "When code block ends I will print <|end|> after it. \n"
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
<|end|>
            """),
        msg(
            'user',
            'Thanks for your help! That is 100% precise for what I meant.'
        ),
        msg(
            'assistant',
            'Anytime! Give me another piece of code you need to explain each line. It is a pleasure to help you!'
        )
    ]
