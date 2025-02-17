import os
import json
import time
from typing import List
from prompt_toolkit import PromptSession, Application, print_formatted_text
from refact import chat_client, cli_main, cli_settings

TRAJ_DIR = os.path.abspath(os.path.join(os.getcwd(), '_trajectories'))

QUESTION = """
Print name for a new file that will contain the current chat. Requirements for the name:
1. It should be snake case with underscores
2. Use double __ between the following sections
3. It should start with name of the project (1-3 words)
4. Continue with initial goal (2-6 words)
5. Finish with success/fail/progress depending on how you see the chat ended
6. Add .json extension

Example:
my_project__rename_my_function1_to_my_function2__success.json

Output only the name and nothing else.
"""


async def think_of_good_filename_and_export(msglist: List[chat_client.Message]):
    os.makedirs(TRAJ_DIR, exist_ok=True)
    traj_contents = sorted(os.listdir(TRAJ_DIR))
    req  = "Chats already saved:\n\n"
    for traj in traj_contents[:20]:
        req += "%s\n" % traj
    if len(traj_contents) > 20:
        req += "...and %d more...\n" % (len(traj_contents) - 20)
    req += "\n"
    req += QUESTION

    try:
        good_name_choices = await chat_client.ask_using_http(
            cli_main.lsp_runner.base_url(),
            [*msglist, chat_client.Message(
                role="user",
                content=req,
            )],
            1,
            cli_settings.args.model,
            verbose=False,
            temperature=0.0,
            max_tokens=100,
        )
        choice0 = good_name_choices[0]
        fn = choice0[-1].content.strip()
    except Exception as e:
        print(f"\n\nFailed to get a good filename using chat: {e}\n\n")
        fn = f"{int(time.time())}.json"

    long_fn = os.path.join(TRAJ_DIR, fn)
    print_formatted_text("\nExport to %r" % long_fn)
    assert " " not in fn
    assert "\n" not in fn

    with open(long_fn, "w") as f:
        json_data = [json.dumps(msg.model_dump(exclude_none=True, exclude_defaults=True), indent=4) for msg in msglist]
        f.write("[\n" + ",\n".join(json_data) + "\n]")
        f.write("\n")
