from typing import Dict, List

from .gpt_toolbox_spad import ScratchpadToolboxGPT
from .gpt35_prompts import msg, \
    add_console_logs, \
    precise_naming_ctxt, \
    completion_ctxt


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

