import os
import time
import termcolor
import asyncio
from refact import chat_client
from pygments import highlight
from pygments.lexers import find_lexer_class_for_filename
from pygments.formatters import TerminalFormatter
from typing import Any, Dict


tools = None

async def ask_chat(messages):
    global tools
    if tools is None:
        tools_turn_on = {"definition"}
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
    )
    return assistant_choices


def sort_out_messages(response_messages):
    tool_call_message = None
    for msg in response_messages:
        print(f"role={msg.role!r}, content={msg.content[:30]!r}")
        if msg.role == "tool":
            assert tool_call_message is None
            tool_call_message = msg
    if tool_call_message is not None:
        print(termcolor.colored(tool_call_message.content, "blue"))
    return tool_call_message


async def test_if_have_definition(symbol: str, should_be: str, should_not_be: str) -> None:
    print(f"testing({symbol!r})")
    initial_messages = [
        chat_client.Message(role="user", content=f"Call definition() for {symbol}"),
        chat_client.Message(role="assistant", content="Alright, here we go", tool_calls=[chat_client.pretend_function_call("definition", {"symbol": symbol})]),
    ]
    assistant_choices = await ask_chat(initial_messages)
    tool_call_message = sort_out_messages(assistant_choices[0][2:])
    assert tool_call_message is not None, "No tool called"

    assert should_be in tool_call_message.content, f"Expected content to contain: {should_be!r}, but it was not found."
    assert should_not_be not in tool_call_message.content, f"Expected content to not contain: {should_not_be!r}, but it was found."
    print(termcolor.colored("PASS", "green"))


if __name__ == '__main__':
    with open("./horrible_experiments.py", "w") as f:
        f.write("class HorribleClass1:\n    pass\n")
    time.sleep(0.01)

    asyncio.run(test_if_have_definition(
        symbol="HorribleClass1",
        should_be="horrible_experiments.py:1",
        should_not_be="No definitions"
    ))
    asyncio.run(test_if_have_definition(
        symbol="HorribleClass2",
        should_be="No definitions",
        should_not_be="horrible_experiments.py"
    ))

    with open("./horrible_experiments.py", "w") as f:
        f.write("\nclass HorribleClass2:\n    pass\n")
    time.sleep(0.01)

    asyncio.run(test_if_have_definition(
        symbol="HorribleClass2",
        should_be="horrible_experiments.py:2",
        should_not_be="No definitions"
    ))
    asyncio.run(test_if_have_definition(
        symbol="HorribleClass1",
        should_be="No definitions",
        should_not_be="horrible_experiments.py"
    ))

    os.remove("./horrible_experiments.py")
    time.sleep(0.01)

    asyncio.run(test_if_have_definition(
        symbol="HorribleClass1",
        should_be="No definitions",
        should_not_be="horrible_experiments.py"
    ))
    asyncio.run(test_if_have_definition(
        symbol="HorribleClass2",
        should_be="No definitions",
        should_not_be="horrible_experiments.py"
    ))
