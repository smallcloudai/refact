import re
import traceback

from refact import chat_client
from step import Step

from pathlib import Path
from typing import Set, List, Optional


RESULT_MARKER = "=====FILES====="


SYSTEM_MESSAGE = f"""
You are Refact Dev, an auto coding assistant.

You'll receive a problem statement from user.
Your aim is to find list of files that should be changed in the process of solving.

Use the following strategy:
1. Read the problem statement carefully.
2. Using definition tool try to explore as much symbols as you can.
3. After symbols exploration add potential changed files to context.
4. If it's needed repeat steps 2 and 3 before you move to step 5.
5. Finally provide one list of files which required for problem solving.

Your final answer should follow the format:
{RESULT_MARKER}
filenames list

Follow this list of rules:
- Base your answer only on tools outputs, do not hallucinate.
- If tool doesn't give you expected result, try another time with different argument.
- First of all use definition tool because it's cheap. If you sure that it's required to list full file use file tool.
- Do not list files that is not related to the problem. Your answer must contain files where we need to make changes!
- For each filename add explanation why you've listed it.
- Problem solving doesn't require test files, so do not try to list, modify or mention them.
"""


MEMORY_MESSAGE = """
I asked you to find files related to the problem above before.
You answered with the following list:
{}

These files are useless for solving the problem.
It's strictly prohibited to use them in your answers and tool calls.
Try to find another files!
"""


class ExploreRepoStep(Step):

    def __init__(self, attempts: int, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self._attempts = attempts

    @property
    def _tools(self) -> Set[str]:
        return {
            "file",
            "definition",
            # "tree",
            # "references",
        }

    @staticmethod
    def _extract_filenames(t: str, repo_path: Optional[Path] = None) -> List[str]:
        pattern = r'\b(?:[a-zA-Z]:\\|/)?(?:[\w-]+[/\\])*[\w-]+\.\w+\b'
        filenames = []

        def _normalize_path(p: str) -> str:
            return "/".join(filter(bool, p.split("/")))

        for filename in re.findall(pattern, t):
            filename = _normalize_path(filename)
            if repo_path is not None:
                filename = filename.replace(_normalize_path(str(repo_path)), "")
                filename = _normalize_path(filename)
            filenames.append(filename)

        return filenames

    async def _single_step(self, message: str) -> str:
        messages = [
            chat_client.Message(role="system", content=SYSTEM_MESSAGE),
            chat_client.Message(role="user", content=message),
        ]

        for step_n in range(self._max_depth):
            print(f"{'-' * 40} step {step_n} {'-' * 40}")
            messages = await self._query(messages)
            last_message = messages[-1]
            if last_message.role == "assistant" \
                    and last_message.content \
                    and RESULT_MARKER in last_message.content:
                return messages[-1].content.split(RESULT_MARKER)[1].strip()
        raise RuntimeError(f"can't produce result with {self._max_depth} steps")

    async def process(self, problem_statement: str, repo_path: Path, **kwargs) -> str:
        results = []

        def _formatted_filenames() -> str:
            filenames = {
                f for r in results
                for f in self._extract_filenames(r, repo_path)
            }
            return "\n".join([f"- {f}" for f in filenames])

        for attempt_n in range(self._attempts):
            print(f"{'=' * 40} attempt {attempt_n} {'=' * 40}")
            message = problem_statement
            filenames_list_text = _formatted_filenames()
            if filenames_list_text:
                message = f"{message}\n\n{MEMORY_MESSAGE.format(filenames_list_text)}"
            try:
                results.append(await self._single_step(message))
            except Exception as e:
                print(f"exception in {self._single_step.__name__}: {str(e) or traceback.format_exc()}, continue")
                continue
        result = _formatted_filenames()
        if result:
            return result
        raise RuntimeError(f"can't produce result with {self._attempts} attempts")
