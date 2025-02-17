import os
import time
import json
import termcolor
import asyncio
from refact import chat_client
from pygments import highlight
from pygments.lexers import find_lexer_class_for_filename
from pygments.formatters import TerminalFormatter
from typing import Any, Dict, List, Set


tools = None

async def ask_chat(messages):
    global tools
    if tools is None:
        tools = await chat_client.tools_fetch_and_filter(base_url="http://127.0.0.1:8001/v1", tools_turn_on=None)
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


async def test_if_located_right(
    question: str,
    expected_roles: Dict[str, str],
    # definitions_complete_list: List[str],
) -> None:
    initial_messages = [
        chat_client.Message(role="user", content=f"{question}"),
        chat_client.Message(role="assistant", content="Alright, here we go:", tool_calls=[chat_client.pretend_function_call("locate", {"problem_statement": question})]),
    ]
    assistant_choices = await ask_chat(initial_messages)
    messages = assistant_choices[0]
    # tool_call_message = sort_out_messages(messages[2:])  # this also prints
    # tool_call_message = xxx
    # print(termcolor.colored(xxx, "blue"))
    hist = chat_client.print_messages(messages)
    open("aaa2.txt", "w").write("\n".join(hist))
    quit()

    tool_json_split = tool_call_message.split("💿")
    assert len(tool_json_split) == 2
    tool_json = json.loads(tool_json_split[0])
    for fn, d in tool_json.items():
        print(fn)
        if expected_role := expected_roles.get(fn):
            assert expected_role == d["WHY_CODE"]
            assert d["RELEVANCY"] == 5
            print(termcolor.colored("    %s" % d["WHY_CODE"], "green"))
        else:
            print(termcolor.colored("    %s" % d["WHY_CODE"], "red"))

    assert tool_call_message is not None, "No tool called"

    # assert should_be in tool_call_message.content, f"Expected content to contain: {should_be!r}, but it was not found."
    # assert should_not_be not in tool_call_message.content, f"Expected content to not contain: {should_not_be!r}, but it was found."
    # print(termcolor.colored("PASS", "green"))


if __name__ == '__main__':
    # asyncio.run(test_if_located_right(
    #     question="find Goat in this project and replace it with Iguana",
    #     expected_roles={
    #         "src/ast/alt_testsuite/cpp_goat_library.h": "TOCHANGE",
    #         "src/ast/alt_testsuite/cpp_goat_main.cpp": "TOCHANGE",
    #     },
    # ))
    asyncio.run(test_if_located_right(
        question="check out Goat in this project, can you write a similar test in typescript?",
        expected_roles={
            # "src/ast/alt_testsuite/cpp_goat_library.h": "TOCHANGE",
            # "src/ast/alt_testsuite/cpp_goat_main.cpp": "TOCHANGE",
        },
    ))
    # check out Goat in this project
    # check out Goat in this project, can you write a similar test in typescript?
