import argparse, json, termcolor, time
import asyncio
from datetime import datetime
from refact import chat_client


DUMP_PREFIX = datetime.now().strftime("%Y%m%d-%H%M%S")

# MODEL = "gpt-4-turbo"
# MODEL = "gpt-4o"
# MODEL = "gpt-3.5-turbo-1106"  # $1, multi call works
# MODEL = "gpt-3.5-turbo-0125"    # $0.50
MODEL = "gpt-3.5-turbo"    # $0.50


SYSTEM_PROMPT = """
You are Refact Chat, a coding assistant.

Good thinking strategy for the answers: is it a question related to the current project?
Yes => collect the necessary context using search, definition and references using up to 5 tools calls in parallel, or just do what the user tells you.
No => answer the question without calling any tools.

Explain your plan briefly before calling the tools in parallel.

IT IS FORBIDDEN TO JUST CALL TOOLS WITHOUT EXPLAINING. EXPLAIN FIRST! USE TOOLS IN PARALLEL!
"""


async def single_test(ask_this, *, tools_must_be):
    messages: [chat_client.Message] = [
        chat_client.Message(role="system", content=SYSTEM_PROMPT),
        chat_client.Message(role="user", content=ask_this),
    ]

    N = 1
    tools_turn_on = {"definition", "references", "compile", "memorize", "file"}
    tools = await chat_client.tools_fetch_and_filter(base_url="http://127.0.0.1:8001/v1", tools_turn_on=tools_turn_on)
    assistant_choices = await chat_client.ask_using_http(
        "http://127.0.0.1:8001/v1",
        messages,
        N,
        MODEL,
        tools=tools,
        verbose=False,
        temperature=0.3,
        stream=False,
        max_tokens=2048,
    )
    assert(len(assistant_choices)==N)
    messages = assistant_choices[0]
    bad = (not not messages[-1].tool_calls) != tools_must_be
    color = "red" if bad else "green"
    content = messages[-1].content if messages[-1].content else "no content"
    content = content[:19]
    if messages[-1].tool_calls:
        # calls_str = ", ".join([x.function.name for x in messages[-1].tool_calls])
        calls_str = ""
        for call in messages[-1].tool_calls:
            calls_str += f" {call.function.name}({call.function.arguments})"
    else:
        calls_str = "no_calls"
    print("%-40s %-20s %s" % (ask_this.replace("\n", "\\n")[:39], termcolor.colored(content, "blue"), termcolor.colored(calls_str, color)))


async def all_tests():
    print("model is %s" % MODEL)
    print("---- must be no calls ----")
    await single_test(ask_this="What is the meaning of life?", tools_must_be=False)
    await single_test(ask_this="What is your name?", tools_must_be=False)
    await single_test(ask_this="Explain string theory", tools_must_be=False)
    await single_test(ask_this="Write pygame example", tools_must_be=False)
    await single_test(ask_this="```\n@validator('input')\n```\nWhy is this outdated in fastapi?", tools_must_be=False)
    print("---- must be calls ----")
    await single_test(ask_this="What is Frog?", tools_must_be=True)
    await single_test(ask_this="Why is there type conversion service in this project?", tools_must_be=True)
    await single_test(ask_this="list methods of ConversionService", tools_must_be=True)
    await single_test(ask_this="explain `public class ReadableBytesTypeConverter implements FormattingTypeConverter<CharSequence, Number, ReadableBytes>`", tools_must_be=True)


if __name__ == "__main__":
    asyncio.run(all_tests())
