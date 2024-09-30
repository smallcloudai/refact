import asyncio
import os
import sys
import argparse
import requests
import random
import termcolor
from typing import Any

from prompt_toolkit import PromptSession, Application, print_formatted_text
from prompt_toolkit.key_binding import KeyBindings
from prompt_toolkit.history import FileHistory
from prompt_toolkit.completion import Completer, Completion
from prompt_toolkit.layout import Layout, CompletionsMenu, Float
from prompt_toolkit.layout.containers import HSplit, VSplit, Window, FloatContainer, ConditionalContainer
from prompt_toolkit.layout.controls import FormattedTextControl
from prompt_toolkit.formatted_text import FormattedText
from prompt_toolkit.widgets import TextArea

from refact.chat_client import Message
from refact.cmdline.inspect import inspect_app, open_label
from refact.cmdline.streaming import ask_chat, print_response, get_response_box
from refact.cmdline.streaming import stop_streaming, is_not_streaming_condition, is_streaming, start_streaming
from refact.cmdline.streaming import add_streaming_message
from refact.cmdline.app_switcher import start_app, exit_all_apps, push_app
from refact.cmdline import statusbar, settings
from refact.lsp_runner import LSPServerRunner


lsp = None


async def answer_question_in_arguments(settings, arg_question):
    add_streaming_message(Message(role="user", content=arg_question))
    await ask_chat(settings.model)


tips_of_the_day = '''
Ask anything: "does this project have SQL in it?"
Ask anything: "summarize README"
Refact Agent is essentially its tools, ask: "what tools do you have?"
'''.strip().split('\n')


async def welcome_message(settings: settings.CmdlineSettings, tip: str):
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
    stop_streaming()
    event.current_buffer.reset()


@kb.add('c-d')
def exit_(event):
    exit_all_apps()


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
    user_input = buffer.text
    if user_input.lower() in ('exit', 'quit'):
        exit_all_apps()
        return

    if user_input[0] == "?":
        label = user_input[1:]
        if open_label(label):
            push_app(inspect_app())
        else:
            print_formatted_text(f"\nchat> {user_input}")
            print_formatted_text(FormattedText([("#ff3333", f"label {label} not found")]))
        return

    print_response(f"\nchat> {user_input}")

    if user_input.strip() == '':
        return

    if is_streaming():
        return

    start_streaming()

    print_response("\n")
    add_streaming_message(Message(role="user", content=user_input))

    async def asyncfunc():
        await ask_chat(settings.settings.model)

    loop = asyncio.get_event_loop()
    loop.create_task(asyncfunc())


async def chat_main():
    global lsp

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

    settings.cli_yaml = settings.load_cli_or_auto_configure()
    app.editing_mode = settings.cli_yaml.get_editing_mode()

    refact_args = [
        os.path.join(os.path.dirname(__file__), "..", "bin", "refact-lsp"),
        "--address-url", settings.cli_yaml.address_url,
        "--api-key", settings.cli_yaml.api_key,
    ]
    if settings.cli_yaml.insecure_ssl:
        refact_args.append("--insecure-ssl")
    if settings.cli_yaml.basic_telemetry:
        refact_args.append("--basic-telemetry")
    if settings.cli_yaml.experimental:
        refact_args.append("--experimental")
    if settings.cli_yaml.ast:
        refact_args.append("--ast")
        refact_args.append("--ast-max-files")
        refact_args.append(str(settings.cli_yaml.ast_max_files))
    if settings.cli_yaml.vecdb:
        refact_args.append("--vecdb")
        refact_args.append("--vecdb-max-files")
        refact_args.append(str(settings.cli_yaml.vecdb_max_files))
    if args.path_to_project:
        refact_args.append("--workspace-folder")
        refact_args.append(args.path_to_project)
    lsp = LSPServerRunner(
        refact_args,
        wait_for_ast_vecdb=False,
        refact_lsp_log=None,
        verbose=False
    )

    async with lsp:
        caps = await settings.fetch_caps(lsp.base_url())
        settings.settings = settings.CmdlineSettings(caps, args)

        if settings.settings.model not in caps.code_chat_models:
            known_models = list(caps.code_chat_models.keys())
            print(f"model {settings.settings.model} is unknown, pick one of {known_models}")
            return

        await welcome_message(settings.settings, random.choice(tips_of_the_day))
        statusbar.model_section = f"model {settings.settings.model} context {settings.settings.n_ctx()}"

        if arg_question:
            print(arg_question)
            await answer_question_in_arguments(settings.settings, arg_question)
            return

        asyncio.create_task(statusbar.statusbar_background_task())
        await start_app(app)


history_fn = os.path.expanduser("~/.cache/refact/cli_history")
session: PromptSession = PromptSession(history=FileHistory(history_fn))

tool_completer = ToolsCompleter()
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
    Window(content=get_response_box(), dont_extend_height=True),
    ConditionalContainer(
        content=FloatContainer(content=vsplit, floats=[
            Float(xcursor=True, ycursor=True, content=CompletionsMenu())]
        ),
        filter=is_not_streaming_condition,
    ),
    Window(),
    statusbar.StatusBar(),
])
layout = Layout(hsplit)
app: Application = Application(key_bindings=kb, layout=layout)


def entrypoint():
    """
    `refact` in console runs this function.
    """
    asyncio.run(chat_main())


if __name__ in '__main__':
    asyncio.run(chat_main())
