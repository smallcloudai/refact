from .gpt_toolbox_spad import ScratchpadToolboxGPT
from typing import Dict, List
from .utils import msg


class GptFixBug(ScratchpadToolboxGPT):
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
                "I will provide you the code that contains bugs, which you will need to find and fix."
                "Output the block of rewritten bug-free code. "
                "Do not explain anything! "
                "Firstly, I will provide you the context, which is the whole file. "
                "Then I will provide you the specific piece of code to find and fix bugs. "
                "When the block of code ends you will print <|end|> after it."
                "In case there are no bugs, leave it as it is."
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
            msg('user', ctxt),
            msg('assistant',
                "Thanks for giving me the context. "
                "I understand it. "
                "Please provide me the part of code you need to fix bugs in."
                ),
            msg('user', self.selection)
        ]


class GptFixBugGPT4(GptFixBug):
    def __init__(self, **kwargs):
        super().__init__(
            model_n='gpt-4',
            **kwargs
        )
