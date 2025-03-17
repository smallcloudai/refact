import argparse, json, termcolor, time
import asyncio
from datetime import datetime
from refact import chat_client


DUMP_PREFIX = datetime.now().strftime("%Y%m%d-%H%M%S")

MODEL = "gpt-4o-mini"
# MODEL = "gpt-3.5-turbo-1106"  # $1, multi call works
# MODEL = "gpt-3.5-turbo-0125"    # $0.50
# MODEL = "gpt-3.5-turbo"    # $0.50
# MODEL = "claude-3-5-sonnet"     # XXX: will not work because need to remove 'agentic' attribute from each tool, anthropic complains about that
print("model is %s" % MODEL)


SYSTEM_PROMPT = '''
You are Refact Chat, a coding assistant.

Recognize these situations:
A) Question is unrelated to the curent project => just answer the question.
B) You already know the answer => just answer the question.
C) You already have the code, class, function, file => just answer the question.
D) Question is related to the current project, but you already have all the information to answer => just answer the question.
E) Question is related to the current project and there is not sufficient information to answer => explain your plan, call functions in parallel.
F) User tells you to do something => just do it.

Begin your answer with words, not function call.

IF YOU KNOW THE ASNWER, DON'T CALL ANY FUNCTIONS OR TOOLS.

CALL FUNCTIONS AFTER YOU WROTE YOUR EXPLANATION.

ONLY USE TOOLS AFTER YOU WROTE YOUR PLAN.
'''


mistakes = 0
no_explanation = 0


async def single_test(system_prompt: str, ask_this: str, *, tools_must_be) -> int:
    messages: [chat_client.Message] = [
        chat_client.Message(role="system", content=system_prompt),
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
        provider_name="",
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
    if messages[-1].tool_calls:
        # calls_str = ", ".join([x.function.name for x in messages[-1].tool_calls])
        calls_str = ""
        for call in messages[-1].tool_calls:
            calls_str += f" {call.function.name}({call.function.arguments})"
    else:
        calls_str = "no_calls"
    report = "%s\n%s\n%s" % (ask_this.replace("\n", "\\n"), termcolor.colored(content.replace("\n", "\\n"), "blue"), termcolor.colored(calls_str, color))
    mistakes = (1 if bad else 0)
    no_explanation = (1 if not messages[-1].content else 0)
    return (report, mistakes, no_explanation)


async def all_tests(sp: str, parallel: bool) -> int:
    # print("---- must be no calls ----")
    mistakes, no_explanation = 0, 0
    towait = []
    async def launch_one(future):
        nonlocal mistakes, no_explanation, towait
        if not parallel:
            report, new_mistakes, new_no_explanation = await future
            print(report)
            mistakes += new_mistakes
            no_explanation += new_no_explanation
        else:
            towait.append(future)
    await launch_one(single_test(sp, ask_this="summarize @definition DeltaDeltaChatStreamer", tools_must_be=False))  # make sure to run lsp with --workspace-folder refact-lsp/
    await launch_one(single_test(sp, ask_this="What is the meaning of life?", tools_must_be=False))
    await launch_one(single_test(sp, ask_this="What is your name?", tools_must_be=False))
    await launch_one(single_test(sp, ask_this="Explain string theory", tools_must_be=False))
    await launch_one(single_test(sp, ask_this="Write pygame example", tools_must_be=False))
    await launch_one(single_test(sp, ask_this="```\n@validator('input')\n```\nWhy is this outdated in fastapi?", tools_must_be=False))
    # print("---- must be calls ----")
    await launch_one(single_test(sp, ask_this="What is Frog?", tools_must_be=True))
    await launch_one(single_test(sp, ask_this="Why is there type conversion service in this project?", tools_must_be=True))
    await launch_one(single_test(sp, ask_this="list methods of ConversionService", tools_must_be=True))
    await launch_one(single_test(sp, ask_this="explain `public class ReadableBytesTypeConverter implements FormattingTypeConverter<CharSequence, Number, ReadableBytes>`", tools_must_be=True))
    for wait_me in towait:
        report, new_mistakes, new_no_explanation = await wait_me
        mistakes += new_mistakes
        no_explanation += new_no_explanation
    return (mistakes, no_explanation)


async def main():
    global mistakes, no_explanation
    mistakes, no_explanation = await all_tests(SYSTEM_PROMPT, parallel=False)
    print("MISTAKES: %d" % mistakes)
    print("NO EXPLANATION: %d" % no_explanation)


if __name__ == "__main__":
    asyncio.run(main())
