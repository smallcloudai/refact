from .gpt35_prompts import msg


def detect_bugs():
    return [
        msg(
            'system',
            "You are an AI programming assistant.\n"
            "Follow the user's requirements carefully & to the letter."
        ),
        msg('user',
            'I am a software engineer. '
            'I have a question about one of my scripts. '
            'I am afraid there are some bugs in it. I need you to find all of them and explain and propose a solution. '
            'You need to stick to the following format: you will output a block of code in jsonlines format.'
            'This is how you must format you output:'
            '''
{"code": "BUGGY_CODE_PART_1", "bug": "BUG_1_DESCRIPTION"}
{"code": "BUGGY_CODE_PART_2", "bug": "BUG_2_DESCRIPTION"}
            '''
            'Explain as briefly as possible, do not explain outside of code block. '
            'The output you provide must be decodable using jsonlines format. '
            ),
        msg('assistant',
            'Thank you for detailed description. '
            'Now please provide me this script that might contain bugs. '
            'I will all of potential bugs for you and explain them in the format you have given. '
            )
    ]


def detect_vulnerabilities():
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
            )
    ]


"""
[Title]
Smart IDE Error Highlighter for Critical Bugs

[Description]
You are developing a smart IDE error highlighter that focuses on identifying critical bugs in a given code file. Your task is to analyze the code file line by line and provide comments only for critical errors, aiming for a concise and accurate output.

[Specifications]
Your program should identify and highlight critical bugs in the code file.
Each identified issue should have a clear description and provide short instructions on how to fix it.
The output should be in the JSONLines format, which is decodable and easy to process.
Avoid commenting on non-critical errors, common coding practices, or style conventions.
Focus on highlighting key and obvious issues that can significantly impact the functionality or security of the code.
[JSONLines Format]
Each identified critical bug should be represented as a JSON object with the following fields:

code: The specific code snippet where the critical bug is located.
description: A brief description of the issue and instructions on how to fix it.
[Example Output]
{"code": "    def _messages(self) -> list[dict[str, str]]:, "description": "errors in type annotations"}
{"code": "for call, idx in enumerate(calls_unfiltered):", "description": "Invalid variable assignment"}

[Note]

Be concise and provide comments only for critical bugs that you are absolutely sure about.
Consider the impact of the issue on functionality or security when deciding if it is critical.
Avoid excessive comments on minor issues or subjective coding style preferences.

"""


def code_review():
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
            """
{"code": "from routers import FindRouter", "description": "ModuleNotFoundError: no module named routers"}
            """
        ),
        msg(
            'user',
            'Not valid. You have been told to ignore any kind of import errors!'
        )

    ]
