import os, json, random
import asyncio
from refact import chat_client

from pygments import highlight
from pygments.lexers import PythonLexer
from pygments.formatters import TerminalFormatter


fpath = os.path.join("tests", "emergency_frog_situation", "jump_to_conclusions.py")
fpath = os.path.join("tests", "emergency_frog_situation", "frog.py")

def generate_tool_call(tool_name, tool_arguments):
    random_hex = ''.join(random.choices('0123456789abcdef', k=6))
    tool_call = {
        "id": f"{tool_name}_{random_hex}",
        "function": {
            "arguments": json.dumps(tool_arguments),
            "name": tool_name
        },
        "type": "function"
    }
    return tool_call

symbol = "Frog"
# symbol = "bounce_off_banks"
# symbol = "DeltaDeltaChatStreamer::response_n_choices"

initial_messages = [
chat_client.Message(role="user", content=f"Call references() for {symbol}"),
chat_client.Message(role="assistant", content="Alright, here we go", tool_calls=[generate_tool_call("references", {"symbol": symbol})]),
]

async def ask_chat(messages):
    tools_turn_on = {"definition", "references", "search", "cat"}
    tools = await chat_client.tools_fetch_and_filter(base_url="http://127.0.0.1:8001/v1", tools_turn_on=tools_turn_on)
    assistant_choices = await chat_client.ask_using_http(
        "http://127.0.0.1:8001/v1",
        messages,
        1,
        "gpt-4o-mini",
        tools=tools,
        verbose=False,
        temperature=0.3,
        stream=True,
        max_tokens=2048,
        only_deterministic_messages=True,
        postprocess_parameters={
            "take_floor": 50.0,
        }
    )
    for i, msg in enumerate(assistant_choices[0]):
        print("messages[%d] role=%-15s content=%s" % (i, msg.role, msg.content.replace("\n", "\\n")[:400] if msg.content is not None else "None"))
        if msg.role == "context_file":
            context_files = json.loads(msg.content)
            for fdict in context_files:
                print(fdict["file_name"])
                hl = highlight(fdict["file_content"], PythonLexer(), TerminalFormatter())
                print(hl)


if __name__ == '__main__':
    asyncio.run(ask_chat(initial_messages))
