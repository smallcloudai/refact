import asyncio
import json
import os
import sys
import argparse
import requests
import random
import termcolor
from typing import Dict, Any, Optional, List

from prompt_toolkit import PromptSession, Application, print_formatted_text
from prompt_toolkit.key_binding import KeyBindings
from prompt_toolkit.history import FileHistory
from prompt_toolkit.completion import Completer, Completion
from prompt_toolkit.layout import Layout, CompletionsMenu, Float
from prompt_toolkit.layout.containers import HSplit, VSplit, Window, FloatContainer, ConditionalContainer
from prompt_toolkit.layout.controls import FormattedTextControl
from prompt_toolkit.formatted_text import FormattedText
from prompt_toolkit.widgets import TextArea
from prompt_toolkit.filters import Condition

from refact.chat_client import Message, FunctionDict, ask_using_http, tools_fetch_and_filter
from refact.cmdline_printing import create_box, indent, wrap_tokens, print_header, highlight_text, limit_lines, get_terminal_width, tokens_len, Lines
from refact.cmdline_printing import print_file, print_lines, highlight_text_by_language, set_background_color
from refact.cmdline_markdown import to_markdown
from refact.lsp_runner import LSPServerRunner
from refact import cmdline_statusbar
from refact import cmdline_settings


def find_tool_call(messages: List[Message], id: str) -> Optional[FunctionDict]:
    for message in messages:
        if message.tool_calls is None:
            continue
        for tool_call in message.tool_calls:
            if tool_call.id != id:
                continue
            return tool_call.function
    return None


response_text = ""
language_printing = None


def flush_response():
    global response_text
    global language_printing
    width = get_terminal_width()
    if response_text[:3] == "```":
        if language_printing is None:
            language_printing = response_text[3:]
            if language_printing.strip() != "":
                tab_color = "#3e4957"
                print_formatted_text(FormattedText([
                    (tab_color, " "),
                    (f"bg:{tab_color}", f" {language_printing.strip()} "),
                    (tab_color, ""),
                ]))
        else:
            language_printing = None
    elif language_printing is None:
        print_formatted_text(FormattedText(to_markdown(response_text, width)), end="")
    else:
        bg_color = "#252b37"

        highlighted = highlight_text_by_language(response_text, language_printing.strip())
        wrapped = wrap_tokens(highlighted, width - 2)
        with_background = set_background_color(wrapped, bg_color)

        print_lines(with_background)

    response_box.text = []
    response_text = ""


def print_response(to_print: str):
    global response_text
    for line in to_print.splitlines(True):
        response_text += line
        response_box.text.append(("", line))
        if line[-1] == "\n":
           flush_response()
    app.invalidate()


def print_context_file(json_str: str):
    file = json.loads(json_str)[0]
    content = file["file_content"]
    file_name = file["file_name"]
    # line1 = file["line1"]
    # line2 = file["line2"]

    print_response("\n")
    flush_response()
    print_file(content, file_name)


streaming_messages = []
is_streaming = False
lsp = None


def process_streaming_data(data):
    global streaming_messages
    if "choices" in data:
        choices = data['choices']
        delta = choices[0]['delta']
        content = delta.get('content', None)
        if content is None:
            finish_reason = choices[0]['finish_reason']
            if finish_reason == 'stop':
                print_response("\n")
            return
        if len(streaming_messages) == 0 or streaming_messages[-1].role != "assistant":
            print_response("\n")
            streaming_messages.append(Message(role="assistant", content=content))
        else:
            streaming_messages[-1].content += content
        print_response(content)

    elif "role" in data:
        role = data["role"]
        if role == "user":
            return
        content = data["content"]
        streaming_messages.append(Message(role=role, content=content))
        if role == "context_file":
            print_context_file(content)
            return
        terminal_width = get_terminal_width()
        box = create_box(content, terminal_width - 4, max_height=10)
        indented = indent(box, 2)
        tool_call_id = data["tool_call_id"]
        print_response("\n")
        flush_response()
        function = find_tool_call(streaming_messages, tool_call_id)
        if function is not None:
            print_formatted_text(f"  {function.name}({function.arguments})")
        else:
            print_formatted_text(f"  function is none")
        print_lines(indented)

    elif "subchat_id" in data:
        pass

    else:
        print_response("unknown streaming data:\n%s" % data)


async def ask_chat(model):
    global streaming_messages
    global is_streaming

    N = 1
    for step_n in range(4):
        def callback(data):
            if is_streaming:
                process_streaming_data(data)

        messages = list(streaming_messages)
        tools = await tools_fetch_and_filter(base_url=lsp.base_url(), tools_turn_on=None)
        new_messages = await ask_using_http(
            lsp.base_url(),
            messages,
            N,
            model,
            tools=tools,
            verbose=False,
            temperature=0.3,
            stream=True,
            max_tokens=2048,
            only_deterministic_messages=False,
            callback=callback,
        )
        streaming_messages = new_messages[0]

        if not is_streaming:
            break
        if not streaming_messages[-1].tool_calls:
            break
    is_streaming = False


async def answer_question_in_arguments(settings, arg_question):
    global streaming_messages
    streaming_messages.append(Message(role="user", content=arg_question))
    await ask_chat(settings.model)


tips_of_the_day = '''
Ask anything: "does this project have SQL in it?"
Ask anything: "summarize README"
Refact Agent is essentially its tools, ask: "what tools do you have?"
'''.strip().split('\n')


async def welcome_message(settings: cmdline_settings.CmdlineSettings, tip: str):
    text = f"""
~/.cache/refact/cli.yaml                -- set up this program
~/.cache/refact/bring-your-own-key.yaml -- set up models you want to use
~/.cache/refact/integrations.yaml       -- set up github, jira, make, gdb, and other tools, including which actions require confirmation
~/.cache/refact/privacy.yaml            -- which files should never leave your computer
Project: {settings.project_path}
To exit, type 'exit' or Ctrl+D. {tip}.
"""
    print(termcolor.colored(text.strip(), "white", None, ["dark"]))

kb = KeyBindings()


@kb.add('escape', 'enter')
def _(event):
    event.current_buffer.insert_text('\n')


@kb.add('enter')
def _(event):
    event.current_buffer.validate_and_handle()


@kb.add('c-c')
def _(event):
    global is_streaming
    is_streaming = False
    event.current_buffer.reset()


@kb.add('c-d')
def exit_(event):
    event.app.exit()


@Condition
def is_not_streaming_condition():
    return not is_streaming


class ToolsCompleter(Completer):
    def __init__(self):
        pass

    def get_completions(self, document, complete_event):
        text = document.text
        position = document.cursor_position
        response = get_at_command_completion(lsp.base_url(), text, position)

        completions = response["completions"]
        replace = response["replace"]
        for completion in completions:
            yield Completion(completion, start_position=-position + replace[0])


def get_at_command_completion(base_url: str, query: str, cursor_pos: int) -> Any:
    url = f"{base_url}/at-command-completion"
    post_me = {
        "query": query,
        "cursor": cursor_pos,
        "top_n": 6,
    }
    result = requests.post(url, json=post_me)
    return result.json()


def on_submit(buffer):
    global response_box
    global is_streaming

    user_input = buffer.text
    if user_input.lower() in ('exit', 'quit'):
        app.exit()
        return

    print_response(f"\nchat> {user_input}")

    if user_input.strip() == '':
        return

    if is_streaming:
        return

    is_streaming = True

    print_response("\n")
    streaming_messages.append(Message(role="user", content=user_input))

    async def asyncfunc():
        await ask_chat(cmdline_settings.settings.model)

    loop = asyncio.get_event_loop()
    loop.create_task(asyncfunc())


async def chat_main():
    global streaming_messages
    global lsp
    # global settings
    streaming_messages = []

    args = sys.argv[1:]
    if '--' in args:
        split_index = args.index('--')
        before_minus_minus = args[:split_index]
        after_minus_minus = args[split_index + 1:]
    else:
        before_minus_minus = args
        after_minus_minus = []

    description = 'Refact Agent access using command-line interface'
    parser = argparse.ArgumentParser(description=description)
    parser.add_argument('path_to_project', type=str, nargs='?', help="Path to the project", default=None)
    parser.add_argument('--model', type=str, help="Specify the model to use")
    parser.add_argument('--experimental', type=bool, default=False, help="Enable experimental features, such as new integrations")
    parser.add_argument('question', nargs=argparse.REMAINDER, help="You can continue your question in the command line after --")
    args = parser.parse_args(before_minus_minus)

    arg_question = " ".join(after_minus_minus)

    cmdline_settings.cli_yaml = cmdline_settings.load_cli_or_auto_configure()

    refact_args = [
        os.path.expanduser("~/code/refact-lsp/target/release/refact-lsp"),
        "--address-url", cmdline_settings.cli_yaml.address_url,
        "--api-key", cmdline_settings.cli_yaml.api_key,
    ]
    if cmdline_settings.cli_yaml.insecure_ssl:
        refact_args.append("--insecure-ssl")
    if cmdline_settings.cli_yaml.basic_telemetry:
        refact_args.append("--basic-telemetry")
    if cmdline_settings.cli_yaml.experimental:
        refact_args.append("--experimental")
    if cmdline_settings.cli_yaml.ast:
        refact_args.append("--ast")
        refact_args.append("--ast-max-files")
        refact_args.append(str(cmdline_settings.cli_yaml.ast_max_files))
    if cmdline_settings.cli_yaml.vecdb:
        refact_args.append("--vecdb")
        refact_args.append("--vecdb-max-files")
        refact_args.append(str(cmdline_settings.cli_yaml.vecdb_max_files))
    if args.path_to_project:
        refact_args.append("--workspace-folder")
        refact_args.append(args.path_to_project)
    lsp = LSPServerRunner(
        refact_args,
        wait_for_ast_vecdb=False,
        refact_lsp_log=None,
        verbose=True
    )

    async with lsp:
        caps = await cmdline_settings.fetch_caps(lsp.base_url())
        cmdline_settings.settings = cmdline_settings.CmdlineSettings(caps, args)

        if cmdline_settings.settings.model not in caps.code_chat_models:
            known_models = list(caps.code_chat_models.keys())
            print(f"model {cmdline_settings.settings.model} is unknown, pick one of {known_models}")
            return

        await welcome_message(cmdline_settings.settings, random.choice(tips_of_the_day))
        cmdline_statusbar.model_section = f"model {cmdline_settings.settings.model} context {cmdline_settings.settings.n_ctx()}"

        if arg_question:
            print(arg_question)
            await answer_question_in_arguments(cmdline_settings.settings, arg_question)
            return

        asyncio.create_task(cmdline_statusbar.statusbar_background_task())
        await app.run_async()


history_fn = os.path.expanduser("~/.cache/refact/cli_history")
session = PromptSession(history=FileHistory(history_fn))

tool_completer = ToolsCompleter()
response_box = FormattedTextControl(text=[])
text_area = TextArea(
    height=10,
    multiline=True,
    accept_handler=on_submit,
    completer=tool_completer,
    focusable=True,
    focus_on_click=True,
    history=session.history,
)
vsplit = VSplit([
    Window(content=FormattedTextControl(text="chat> "), width=6),
    text_area,
])
hsplit = HSplit([
    Window(content=response_box, dont_extend_height=True),
    ConditionalContainer(
        content=FloatContainer(content=vsplit, floats=[
            Float(xcursor=True, ycursor=True, content=CompletionsMenu())]
        ),
        filter=is_not_streaming_condition,
    ),
    Window(),
    cmdline_statusbar.StatusBar(),
])
layout = Layout(hsplit)
app = Application(key_bindings=kb, layout=layout)


def entrypoint():
    """
    `refact` in console runs this function.
    """
    asyncio.run(chat_main())


if __name__ in '__main__':
    asyncio.run(chat_main())
