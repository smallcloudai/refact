from refact import chat_client
from step import Step

from pathlib import Path
from typing import Set, List


SYSTEM_EXTRACT_SYMBOLS = """
You're Refact Dev a prefect AI assistant.

Your goal is to explore chat and extract all symbols, variables, identities to enlarge context.
You should answer with list of filenames, symbols ordered by their usefullness.

Format of the answer:

- filename1
- filename2
- filename3
- symbol1
- symbol2
- symbol3
- symbol4
"""


DONE_EXPLORATION = "DONE"
SYSTEM_EXPLORATION = f"""
You're Refact Dev a prefect AI assistant.

Your goal is to explore the repo using workspace_map.
Exploration should give needed and valid context for given problem statement.
Join symbols from last message into one string and pass it to the workspace_map tool.
If you have some filenames, join them as symbols and pass this string as path arg it to the workspace_map tool.
Symbols and paths must be separated by comma and there is no space between them.

Never use workspace_map separate for each symbol/path. For example: workspace_map("symbols":"func1,func2,mycls", "paths":"a.cpp,b.py")
Only add files that really needed for task solving.

When you have result from tool just collect all symbols and filenames to the next step of exploration.
Paths in the answer should contain full path to the files within workspace. Do not start filepath with /.
Do not repeat yourself in your final answer.

Format of the answer:
{DONE_EXPLORATION}
- filepath1
- filepath2
- filepath3
- symbol1
- symbol2
- symbol3
- symbol4
"""


class ExploreRepoStep(Step):

    def __init__(self, attempts: int, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self._attempts = attempts

    async def _chat_iteration(
            self,
            messages: List[chat_client.Message],
            system_message: chat_client.Message,
            tools: Set[str]):
        tools = await chat_client.tools_fetch_and_filter(
            base_url=self._base_url,
            tools_turn_on=tools)
        assistant_choices = await chat_client.ask_using_http(
            self._base_url, [system_message, *messages], 1, self._model_name,
            tools=tools, verbose=True, temperature=0.2,
            stream=False, max_tokens=2048,
            only_deterministic_messages=False,
        )
        return assistant_choices[0][1:]

    @staticmethod
    def _filter_symbols(symbols: str) -> str:
        return "\n".join(
            line for line in symbols.split("\n")
            if "test" not in line
        )

    async def process(self, problem_statement: str, repo_path: Path, **kwargs) -> str:
        extract_tools = set()
        extract_system_message = chat_client.Message(role="system", content=SYSTEM_EXTRACT_SYMBOLS)
        explore_tools = {"workspace_map"}
        explore_system_message = chat_client.Message(role="system", content=SYSTEM_EXPLORATION)

        # initial extract symbols
        messages = [
            chat_client.Message(role="user", content=problem_statement),
        ]
        messages = await self._chat_iteration(messages, extract_system_message, extract_tools)
        symbols = self._filter_symbols(messages[-1].content)

        for _ in range(self._attempts):
            content = "\n\n".join([
                "Problem statement:",
                problem_statement,
                "Context symbols and paths for the problem:",
                symbols,
            ])
            messages = [
                chat_client.Message(role="user", content=content),
            ]

            # extract symbols
            messages = await self._chat_iteration(messages, extract_system_message, extract_tools)

            # explore repo
            for _ in range(3):
                messages = await self._chat_iteration(messages, explore_system_message, explore_tools)
                if messages[-1].content is not None and DONE_EXPLORATION in messages[-1].content:
                    symbols = self._filter_symbols(messages[-1].content.split(DONE_EXPLORATION)[1])
                    break

        return symbols
