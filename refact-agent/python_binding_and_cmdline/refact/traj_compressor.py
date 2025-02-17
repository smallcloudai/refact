import os
import json
from typing import List
from prompt_toolkit import PromptSession, Application, print_formatted_text
from refact import chat_client, cli_main, cli_settings, cli_export


async def trajectory_compressor(msglist: List[chat_client.Message]):
    trajectory = await chat_client.compress_trajectory(cli_main.lsp_runner.base_url(), msglist)
    long_fn = os.path.join(cli_export.TRAJ_DIR, "_compressed.json")
    with open(long_fn, "w") as f:
        json.dump(trajectory, f, indent=4)
