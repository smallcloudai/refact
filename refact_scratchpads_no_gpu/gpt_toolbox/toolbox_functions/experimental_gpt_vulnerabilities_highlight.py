import json

from typing import Dict, List

from refact_scratchpads_no_gpu.gpt_toolbox.gpt_utils import msg, find_substring_positions
from refact_scratchpads_no_gpu.gpt_toolbox.gpt_toolbox_spad import ScratchpadToolboxGPT


class GptDetectVulnerabilitiesHighlightGPT4(ScratchpadToolboxGPT):
    def __init__(self, **kwargs):
        super().__init__(
            model_n='gpt-4-0314',
            supports_stream=False,
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
                'I am a software engineer. '
                'I have a question about one of my scripts. '
                'I am afraid there are some vulnerabilities in it. I need you to find them and explain. '
                'You need to stick to the following format: you will output a block of code in jsonlines format.'
                'This is how you must format you output:'
                '''
    {"code": "VULNERABLE_CODE_PART_1", "vulnerability": "YOUR_VULNERABILITY_1_DESCRIPTION"}
    {"code": "VULNERABLE_CODE_PART_2", "vulnerability": "YOUR_VULNERABILITY_2_DESCRIPTION"}
                '''
                'Explain as briefly as possible, do not explain outside of code block. '
                'The output you provide must be decodable using jsonlines format. '
            ),
            msg('assistant',
                'Thank you for detailed description. '
                'Now please provide me this script that might contain vulnerabilities. '
                'I will find them for you and explain them in the format you have given. '
            ),
            msg('user', self._txt)
        ]

    def _postprocess(self, completion: str) -> str:
        suggestions = [json.loads(c) for c in completion.split('\n')]

        for s in suggestions:
            code = s['code']
            indexes = find_substring_positions(code, self._txt)
            if not indexes:
                self.debuglog('Substring not found')
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
