import argparse, json, termcolor, time
# os.environ["OPENAI_LOG"] = "debug"
# os.environ["OPENAI_LOG_JSON"] = "true"
import asyncio
from datetime import datetime
from refact import chat_client


DUMP_PREFIX = datetime.now().strftime("%Y%m%d-%H%M%S")
DEPTH = 2

# MODEL = "gpt-4-turbo"
# MODEL = "gpt-4o"
MODEL = "gpt-4o-mini"
# MODEL = "claude-3-5-sonnet"


SYSTEM_PROMPT = """
You are Refact Chat, a coding assistant.

Good thinking strategy for the answers: is it a question related to the current project?
Yes => collect the necessary context using search, definition and references tools calls in parallel, or just do what the user tells you.
No => answer the question without calling any tools.

Explain your plan briefly before calling the tools in parallel.

IT IS FORBIDDEN TO JUST CALL TOOLS WITHOUT EXPLAINING. EXPLAIN FIRST! USE TOOLS IN PARALLEL!
"""

PLEASE_WRITE_NOTE2 = """
How many times user has corrected you? Write "Number of correction points N".
Then start each one with "---\n", describe what you (the assistant) did wrong, write "Mistake: ..."
Write documentation to tools or the project in general that will help you next time, describe in detail how tools work, or what the project consists of, write "Documentation: ..."
A good documentation for a tool describes what is it for, how it helps to answer user's question, what applicability criteia were discovered, what parameters work and how it will help the user.
A good documentation for a project describes what folders, files are there, summarization of each file, classes. Start documentation for the project with project name.
After describing all points, call note_to_self() in parallel for each actionable point, generate keywords that should include the relevant tools, specific files, dirs, and put documentation-like paragraphs into text.
"""

PLEASE_WRITE_NOTE = """
How many times did you used a tool incorrectly, so it didn't produce the indended result? Call remember_how_to_use_tools() with this exact format:

CORRECTION_POINTS: N

POINT1 WHAT_I_DID_WRONG: i should have used ... tool call or method or plan ... instead of this tool call or method or plan.
POINT1 FOR_FUTURE_FEREFENCE: when ... [describe situation when it's applicable] use ... tool call or method or plan.

POINT2 WHAT_I_DID_WRONG: ...
POINT2 FOR_FUTURE_FEREFENCE: ...
"""


USER_MESSAGE_BY_DEFAULT = "Summarize class Frog"


async def do_all():
    global DEPTH
    parser = argparse.ArgumentParser()
    parser.add_argument('--start-with', type=str, help='Dump with initial messages')
    parser.add_argument('--user', type=str, help='User message')
    parser.add_argument('--note', action='store_true', help='Generate note')
    parser.add_argument('--stream', action='store_true', help='Stream messages')
    args = parser.parse_args()
    if args.start_with:
        with open(f"_logs/note_logs/{args.start_with}", "r") as f:
            j = json.loads(f.read())
        messages = [chat_client.Message.parse_obj(x) for x in j]
        if messages[-1].role == "assistant" and not messages[-1].tool_calls:
            assert args.user or args.note
            if args.user:
                messages.append(chat_client.Message(role="user", content=args.user))
            else:
                DEPTH = 2
                messages.append(chat_client.Message(role="user", content=PLEASE_WRITE_NOTE))
        else:
            print("PROBABLY BAD HISTORY: last message is not an assistant message without calls, skip adding any user message")
            time.sleep(1)
    else:
        messages = [
            # chat_client.Message(role="system", content="You are a coding assistant. Call tools in parallel for efficiency."),
            chat_client.Message(role="system", content=SYSTEM_PROMPT),
            chat_client.Message(role="user", content=(USER_MESSAGE_BY_DEFAULT if not args.user else args.user)),
        ]

    # This replaces system prompt even with history to be able to tune it
    if messages[0].role != "system":
        messages.insert(0, chat_client.Message(role="system", content=SYSTEM_PROMPT))
    else:
        messages[0] = chat_client.Message(role="system", content=SYSTEM_PROMPT)

    for step_n in range(DEPTH):
        print("-"*40 + " step %d " % step_n + "-"*40)
        N = 1
        # tools_turn_on = {"remember_how_to_use_tools"} if args.note else {"definition", "references", "compile", "memorize", "file"}
        # claude requires non-empty tools each step
        tools_turn_on = {"definition", "references", "search", "cat"}
        tools = await chat_client.tools_fetch_and_filter(base_url="http://127.0.0.1:8001/v1", tools_turn_on=tools_turn_on)
        assistant_choices = await chat_client.ask_using_http(
            "http://127.0.0.1:8001/v1",
            messages,
            N,
            MODEL,
            provider_name="",
            tools=tools,
            verbose=True,
            temperature=0.3,
            stream=args.stream,
            max_tokens=2048,
            only_deterministic_messages=(args.note and step_n==1),
        )
        assert(len(assistant_choices)==N)
        messages = assistant_choices[0]
        with open(f"_logs/note_logs/{DUMP_PREFIX}.json", "w") as f:
            json_data = [json.dumps(msg.dict(), indent=4) for msg in messages]
            f.write("[\n" + ",\n".join(json_data) + "\n]")
            f.write("\n")
        if not messages[-1].tool_calls:
            break


if __name__ == "__main__":
    asyncio.run(do_all())
