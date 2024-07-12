from refact import chat_client
from step import Step

from pathlib import Path
from typing import Set


RESULT_MARKER = "=====FILENAMES====="


SYSTEM_MESSAGE = f"""
You are Refact Dev, an auto coding assistant.

You'll receive a problem statement from user.
Your aim is to find list of files that should be changed in the process of solving.

Use the following strategy:
1. Read the problem statement carefully.
2. Using definition tool try to explore as much symbols as you can.
3. After symbols exploration add potential changed files to context.
4. If it's needed repeat steps 2 and 3 before you move to step 5. 
5. Finally provide list of files which required for problem solving.

Your final answer should follow the format:
{RESULT_MARKER}
filenames list

Follow this list of rules:
- Explain your exploration process before using tools.
- Base your answer only on tools outputs, do not hallucinate.
- If tool doesn't give you expected result, try another time with different argument.
- First of all use definition tool because it's cheap. If you sure that it's required to list full file use file tool.
- Do not list files that is not related to the problem. Your answer must contain files where we need to make changes!
- For each filename add explanation why you've listed it.
- Problem solving doesn't require test files, so do not try to list, modify pr mention them.
"""


class SetTaskStep(Step):
    @property
    def _tools(self) -> Set[str]:
        return {
            "file",
            "definition",
            # "tree",
            # "references",
        }

    async def process(self, problem_statement: str, repo_path: Path, **kwargs) -> str:
        messages = [
            chat_client.Message(role="system", content=SYSTEM_MESSAGE),
            chat_client.Message(role="user", content=problem_statement),
        ]

        for step_n in range(self._max_depth):
            print(f"{'-' * 40} step {step_n} {'-' * 40}")
            tools = await chat_client.tools_fetch_and_filter(
                base_url=self._base_url,
                tools_turn_on=self._tools)
            assistant_choices = await chat_client.ask_using_http(
                self._base_url, messages, 1, self._model_name,
                tools=tools, verbose=True, temperature=self._temperature,
                stream=False, max_tokens=2048,
                only_deterministic_messages=False,
            )

            messages = assistant_choices[0]
            last_message = messages[-1]
            if last_message.role == "assistant" \
                    and last_message.content \
                    and RESULT_MARKER in last_message.content:
                # probably we need to split by RESULT_MARKER and get second part
                return messages[-1].content.replace(RESULT_MARKER, "")
        raise RuntimeError(f"can't produce task with {self._max_depth} steps")
