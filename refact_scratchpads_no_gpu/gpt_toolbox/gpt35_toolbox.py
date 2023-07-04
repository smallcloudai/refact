from typing import Dict, List

import ujson as json
from termcolor import colored

from .gpt_toolbox_spad import ScratchpadChatGPT
from .gpt35_prompts import msg, \
    make_code_shorter_ctxt, fix_bug_ctxt, \
    explain_code_block_ctxt, \
    add_console_logs, \
    precise_naming_ctxt, \
    comment_each_line, \
    completion_ctxt

from .gpt4_prompts import code_review

from .utils import code_block_postprocess, find_substring_positions


class ScratchpadCompletion(ScratchpadChatGPT):
    def _messages(self) -> List[Dict[str, str]]:
        cursor0, _, ctxt = self.trim_context()
        ctxt = ctxt[:cursor0] + '<|complete-me|>' + ctxt[cursor0:]
        return [
            *completion_ctxt(),
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


class ScratchpadDetectBugsHighlight(ScratchpadChatGPT):
    def __init__(self, model_n="gpt3.5-turbo-0301", supports_stream=False, **kwargs):
        super().__init__(
            model_n=model_n,
            supports_stream=supports_stream,
            **kwargs
        )

    def _messages(self) -> List[Dict[str, str]]:
        return [
            *code_review(),
            msg('user', self._txt)
        ]

    def _postprocess(self, completion: str) -> str:
        print(colored(f'Completion:\n{completion}', 'yellow'))
        suggestions = []
        for line in completion.splitlines():
            if not line.strip():
                continue
            try:
                suggestions.append(json.loads(line))
            except Exception as e:
                print(e)
        for s in suggestions:
            code = s['code']
            indexes = find_substring_positions(code, self._txt)
            if not indexes:
                print('Substring not found')
                continue
            s_start, s_end = indexes
            self._txt = \
                self._txt[:s_start] + \
                f'\n<BUG>' \
                f'\nDESC: {s["description"]}\n' \
                f'{self._txt[s_start:s_end]}' \
                f'\n</BUG>' + \
                self._txt[s_end:]
        return self._txt


class ScratchpadMakeCodeShorter(ScratchpadChatGPT):
    def _messages(self) -> List[Dict[str, str]]:
        _, _, ctxt = self.trim_context()
        return [
            *make_code_shorter_ctxt(),
            msg('user', ctxt),
            msg('assistant',
                "Thanks for giving me the context. "
                "I understand it. "
                "Please provide me the part of code you need to simplify."
                ),
            msg('user', self.selection)
        ]


class ScratchpadFixBug(ScratchpadChatGPT):
    def _messages(self) -> List[Dict[str, str]]:
        _, _, ctxt = self.trim_context()
        return [
            *fix_bug_ctxt(),
            msg('user', ctxt),
            msg('assistant',
                "Thanks for giving me the context. "
                "I understand it. "
                "Please provide me the part of code you need to fix bugs in."
                ),
            msg('user', self.selection)
        ]


class ScratchpadExplainCodeBlock(ScratchpadChatGPT):
    def _messages(self) -> List[Dict[str, str]]:
        _, _, ctxt = self.trim_context()
        return [
            *explain_code_block_ctxt(),
            msg('user', ctxt),
            msg('assistant',
                "Thanks for giving me the context. "
                "I understand it. "
                "Please provide me the part of code you need to explain in a block."
                ),
            msg('user', self.selection)
        ]

    def _postprocess(self, completion: str) -> str:
        completion = code_block_postprocess(completion)
        return self._txt[:self.cursor1] + '\n' + completion + self._txt[self.cursor1:]


class ScratchpadAddConsoleLogs(ScratchpadChatGPT):
    def _messages(self) -> List[Dict[str, str]]:
        return [
            *add_console_logs(),
            msg('user', self.selection)
        ]


class ScratchpadPreciseNaming(ScratchpadChatGPT):
    def _messages(self) -> List[Dict[str, str]]:
        _, _, ctxt = self.trim_context()
        return [
            *precise_naming_ctxt(),
            msg('user', ctxt),
            msg('assistant',
                "Thanks for giving me the context. "
                "I understand it. "
                "Please provide me the part of code you need to fix naming in."
                ),
            msg('user', self.selection)
        ]


class ScratchpadCommentEachLine(ScratchpadChatGPT):
    def _messages(self) -> List[Dict[str, str]]:
        return [
            *comment_each_line(),
            msg('user', self.selection)
        ]
