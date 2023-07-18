import json

from typing import Dict, List

from refact_scratchpads_no_gpu.gpt_toolbox.gpt_toolbox_spad import ScratchpadToolboxGPT
from refact_scratchpads_no_gpu.gpt_toolbox.gpt_utils import msg, find_substring_positions


class ScratchpadCodeReviewHighlightGPT4(ScratchpadToolboxGPT):
    def __init__(self, **kwargs):
        super().__init__(
            model_n='gpt-4',
            supports_stream=False,
            timeout=120,
            **kwargs
        )

    def _messages(self) -> List[Dict[str, str]]:
        return [
            msg(
                'system',
                "You are an AI programming assistant.\n"
                "Follow the user's requirements carefully & to the letter."
            ),
            msg('user',
                '''
You are a code reviewer.
Follow my instructions carefully & to the letter.

You are to receive a single code file.
It contain imports from other files that are present in the project, but you cannot see them.
That's why you must not highlight errors that are connected to the imports to not commit false-positive errors.

Your assignment is:
1. Carefully read code line by line up to the end.
2. Find all possible errors that interrupt code runtime (except the cases listed above)
3. For each found error you will suggest a comment in the following format:
{"code": "    def _messages(self) -> list[dict[str, str]]:", "description": "errors in type annotations"}
{"code": "for call, idx in enumerate(calls_unfiltered):", "description": "Invalid variable assignment"}

FIELDS DESCRIPTION:
- code: the code you found issue in
- description: extremely brief description of the issue and short instructions hints how to fix it

Guidelines:
Explain yourself as briefly and clear as possible, do not explain outside of code block.
The output you provide must be decodable using jsonlines format.
Do not highlight any error that is anyhow connected to imports!
'''
            ),
            msg(
                'user',
                """
from routers import FindRouter

if __name__ == "__main__":
    from argparse import ArgumentParser
    parser = ArgumentParser()
"""
            ),
            msg(
                'assistant',
                """{"code": "from routers import FindRouter", "description": "ModuleNotFoundError: no module named routers"}"""
            ),
            msg(
                'user',
                'Not valid. You have been told to ignore any kind of import errors!'
            ),
            msg(
                'assistant',
                "Sorry for the confusion. Give me another example."
            ),
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
