from typing import Dict, List

import ujson as json
from termcolor import colored

from .gpt_toolbox_spad import ScratchpadToolboxGPT
from .gpt35_prompts import msg, \
    explain_code_block_ctxt, \
    add_console_logs, \
    precise_naming_ctxt, \
    comment_each_line, \
    completion_ctxt

from .gpt4_prompts import code_review

from .utils import code_block_postprocess, find_substring_positions


class ScratchpadCompletion(ScratchpadToolboxGPT):
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




class ScratchpadExplainCodeBlock(ScratchpadToolboxGPT):
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


class ScratchpadAddConsoleLogs(ScratchpadToolboxGPT):
    def _messages(self) -> List[Dict[str, str]]:
        return [
            *add_console_logs(),
            msg('user', self.selection)
        ]


class ScratchpadPreciseNaming(ScratchpadToolboxGPT):
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


class ScratchpadCommentEachLine(ScratchpadToolboxGPT):
    def _messages(self) -> List[Dict[str, str]]:
        return [
            *comment_each_line(),
            msg('user', self.selection)
        ]
