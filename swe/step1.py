from refact import chat_client
from step import Step

from typing import Set


TASK_MESSAGE_MARKER = "=====TASK====="
SYSTEM_MESSAGE = f"""
You are Refact Dev, an auto coding assistant.

You'll receive a problem statement from user.
Your aim is to rewrite it as a task for developer.

Use the following strategy:
1. Read the problem statement carefully.
2. Use given tools to explore code related to the issue and discuss how to solve it: you must find the real cause of the problem.
3. Set a task for developer that doesn't contain redundant information from the problem statement. Task should be started from {TASK_MESSAGE_MARKER}.

Your final answer should be in the following format:
{TASK_MESSAGE_MARKER}
todo explanation
task-related filenames list

Do not try to solve the issue yourself.
Your task must contain list of files that should be changed in the process of solving.
Each file name should contain full path to the file within the repo.

Explain your plan briefly before calling the tools in parallel.
IT IS FORBIDDEN TO JUST CALL TOOLS WITHOUT EXPLAINING. EXPLAIN FIRST! USE TOOLS IN PARALLEL!
"""


class SetTaskStep(Step):
    @property
    def _tools(self) -> Set[str]:
        return {
            "file",
            "definition",
            "references",
        }

    async def process(self, problem_statement: str, **kwargs) -> str:
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
                    and TASK_MESSAGE_MARKER in last_message.content:
                # probably we need to split by TASK_MESSAGE_MARKER and get second part
                return messages[-1].content.replace(TASK_MESSAGE_MARKER, "")
        raise RuntimeError(f"can't produce task with {self._max_depth} steps")
