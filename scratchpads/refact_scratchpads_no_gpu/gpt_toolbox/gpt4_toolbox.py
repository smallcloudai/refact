from typing import *

import ujson as json

from .gpt_toolbox_spad import ScratchpadChatGPT

from .gpt35_prompts import msg
from .gpt4_prompts import detect_vulnerabilities, detect_bugs, code_review

from .utils import find_substring_positions
from .gpt35_toolbox import \
    ScratchpadMakeCodeShorter, \
    ScratchpadFixBug, \
    ScratchpadExplainCodeBlock, \
    ScratchpadCompletion, ScratchpadDetectBugsHighlight


class ScratchpadMakeCodeShorterGPT4(ScratchpadMakeCodeShorter):
    def __init__(self, **kwargs):
        super().__init__(
            model_n='gpt-4',
            **kwargs
        )


class ScratchpadFixBugGPT4(ScratchpadFixBug):
    def __init__(self, **kwargs):
        super().__init__(
            model_n='gpt-4',
            **kwargs
        )


class ScratchpadExplainCodeBlockGPT4(ScratchpadExplainCodeBlock):
    def __init__(self, **kwargs):
        super().__init__(
            model_n='gpt-4',
            **kwargs
        )


class ScratchpadCompletionGPT4(ScratchpadCompletion):
    def __init__(self, **kwargs):
        super().__init__(
            model_n='gpt-4',
            **kwargs
        )

    def _postprocess(self, completion: str) -> str:
        return self._txt[:self.cursor0] + completion + self._txt[self.cursor1:]



# --- UNFINISHED BELOW THIS LINE ----

class ScratchpadDetectBugsHighlightGPT4(ScratchpadDetectBugsHighlight):
    def __init__(self, **kwargs):
        super().__init__(
            model_n='gpt-4',
            supports_stream=False,
            **kwargs
        )


class ScratchpadDetectVulnerabilitiesHighlight(ScratchpadChatGPT):
    def __init__(self, **kwargs):
        super().__init__(
            model_n='gpt-4-0314',
            supports_stream=False,
            **kwargs
        )

    def _messages(self) -> List[Dict[str, str]]:
        return [
            *detect_vulnerabilities(),
            msg('user', self._txt)
        ]

    def _postprocess(self, completion: str) -> str:
        suggestions = [json.loads(c) for c in completion.split('\n')]

        for s in suggestions:
            code = s['code']
            indexes = find_substring_positions(code, self._txt)
            if not indexes:
                print('Substring not found')
                continue

            s_start, s_end = indexes
            self._txt = \
                self._txt[:s_start] + \
                f'\n<VULNERABLE>' \
                f'\nDESC: {s["vulnerability"]}\n' \
                f'{self._txt[s_start:s_end]}' \
                f'\n</VULNERABLE>' + \
                self._txt[s_end:]

        return self._txt


class ScratchpadCodeReviewHighlight(ScratchpadChatGPT):
    def __init__(self, **kwargs):
        super().__init__(
            model_n='gpt-4',
            supports_stream=False,
            timeout=120,
            **kwargs
        )

    def _messages(self) -> List[Dict[str, str]]:
        return [
            *code_review(),
            msg('user', self._txt)
        ]

    def _postprocess(self, completion: str) -> str:
        suggestions = [json.loads(c) for c in completion.split('\n')]

        for s in suggestions:
            code = s['code']
            indexes = find_substring_positions(code, self._txt)
            if not indexes:
                print('Substring not found')
                continue

            s_start, s_end = indexes
            self._txt = \
                self._txt[:s_start] + \
                f'\n<COMMENT>' \
                f'\nDESC: {s["description"]}\n' \
                f'SCORE: {s["critical_score"]}\n' \
                f'{self._txt[s_start:s_end]}' \
                f'\n</COMMENT>' + \
                self._txt[s_end:]

        return self._txt
