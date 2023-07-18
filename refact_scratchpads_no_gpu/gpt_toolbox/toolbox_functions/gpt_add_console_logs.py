from typing import Dict, List

from refact_scratchpads_no_gpu.gpt_toolbox.gpt_utils import msg
from refact_scratchpads_no_gpu.gpt_toolbox.gpt_toolbox_spad import ScratchpadToolboxGPT


class GptAddConsoleLogs(ScratchpadToolboxGPT):
    def _messages(self) -> List[Dict[str, str]]:
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
            """```
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
```"""),
            msg('user', self.selection)
        ]
