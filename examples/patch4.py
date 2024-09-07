import json
import asyncio
from datetime import datetime
from refact import chat_client


DUMP_PREFIX = datetime.now().strftime("%Y%m%d-%H%M%S")
DEPTH = 2

# MODEL = "gpt-4-turbo"
# MODEL = "gpt-4o"
# MODEL = "gpt-3.5-turbo-1106"  # $1, multi call works
# MODEL = "gpt-3.5-turbo-0125"    # $0.50
MODEL = "gpt-3.5-turbo"


test_message = """
@file metal_snake.py

pls fix this:
```
Traceback (most recent call last):
  File "metal_snake.py", line 6, in <module>
    import sys, impotlib, os
ModuleNotFoundError: No module named 'impotlib'
```
"""


async def do_all():
    messages = [
        chat_client.Message(role="user", content=test_message),
    ]

    for step_n in range(DEPTH):
        print("-"*40 + " step %d " % step_n + "-"*40)
        N = 1
        tools = await chat_client.tools_fetch_and_filter(base_url="http://127.0.0.1:8001/v1", tools_turn_on=None)
        assistant_choices = await chat_client.ask_using_http(
            "http://127.0.0.1:8001/v1",
            messages,
            N,
            MODEL,
            tools=tools,
            verbose=True,
            temperature=0.3,
            stream=False,
            max_tokens=2048,
            only_deterministic_messages=False,
        )
        assert(len(assistant_choices)==N)
        messages = assistant_choices[0]
        with open(f"note_logs/patch4_{DUMP_PREFIX}.json", "w") as f:
            json_data = [json.dumps(msg.dict(), indent=4) for msg in messages]
            f.write("[\n" + ",\n".join(json_data) + "\n]")
            f.write("\n")
        if not messages[-1].tool_calls:
            break


if __name__ == "__main__":
    asyncio.run(do_all())
