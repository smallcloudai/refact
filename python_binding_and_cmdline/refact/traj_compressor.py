import os
import json
from typing import List
from prompt_toolkit import PromptSession, Application, print_formatted_text
from refact import chat_client, cli_main, cli_settings, cli_export


QUESTION = """
Compress the chat above.

Some guidelines:

1. Always prefer specifics over generic phrases. Write file names, symbol names, folder names, actions, facts, user attitude
towards entities in the project. If something is junk according to the user, that's the first priority to remember.
2. The first message in the chat is the goal. Summarize it up to 15 words, always prefer specifics.
3. The most important part is decision making by assistant. What new information assistant has learned? Skip the plans,
fluff, explanations for the user. Keep the line from evidence to decision when summarizing the actions of the assistant.
Write just one sentense. Prefer specifics and facts.
4. Each tool call should be a separate record. Write all the parameters. Summarize facts about output of a tool, especially the facts
useful for the goal.
5. Skip unsuccesful calls that are later corrected, doesn't matter.
7. The last line is outcome, pick SUCCESS/FAIL/PROGRESS

Output format:

[
["goal", "rename my_function1 to my_function2"],
["thinking", "there are definition(), search() and locate() tools, all can be used to find my_function1, system prompt says I need to start with locate()"],
["locate(problem_statement=\"rename my_function1 to my_function2\")", "file my_script.py (1337 lines) has my_function1 on line 42"],
["thinking", "I can rewrite my_function1 inside my_script.py using 📍-notation, so I'll do that"],
["patch(path=\"my_script\", tickets=\"000\")", "The output of patch() has 15 lines_add and 15 lines_remove, confirming the operation"],
["outcome", "SUCCESS"]
]

Write only the json and nothing else.
"""


async def trajectory_compressor(msglist: List[chat_client.Message]):
    json_choices = await chat_client.ask_using_http(
        cli_main.lsp_runner.base_url(),
        [*msglist, chat_client.Message(
            role="user",
            content=QUESTION,
        )],
        1,
        cli_settings.args.model,
        verbose=False,
        temperature=0.3,
        max_tokens=1000,
    )
    choice0 = json_choices[0]
    choice0_last = choice0[-1]
    print_formatted_text("\n%s" % choice0_last.content)

    long_fn = os.path.join(cli_export.TRAJ_DIR, "_compressed.json")
    with open(long_fn, "w") as f:
        json_data = [json.dumps(msg.model_dump(exclude_none=True, exclude_defaults=True), indent=4) for msg in choice0]
        f.write("[\n" + ",\n".join(json_data) + "\n]")
        f.write("\n")
