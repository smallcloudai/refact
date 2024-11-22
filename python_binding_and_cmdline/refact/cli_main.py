import asyncio
import os
import sys
import argparse
import requests
import random
import termcolor
import base64
import glob
import json
from typing import Any, Optional

from prompt_toolkit import PromptSession, Application, print_formatted_text
from prompt_toolkit.key_binding import KeyBindings
from prompt_toolkit.history import FileHistory
from prompt_toolkit.completion import Completer, Completion
from prompt_toolkit.layout import Layout, CompletionsMenu, Float
from prompt_toolkit.layout.containers import HSplit, VSplit, Window, FloatContainer, ConditionalContainer
from prompt_toolkit.layout.controls import FormattedTextControl
from prompt_toolkit.formatted_text import FormattedText
from prompt_toolkit.widgets import TextArea

from refact.cli_inspect import inspect_app, open_label
from refact.cli_streaming import the_chatting_loop, print_response, get_entertainment_box
from refact.cli_streaming import stop_streaming, is_not_streaming_condition, start_streaming
from refact import chat_client
from refact import cli_streaming
from refact import cli_printing
from refact import cli_export
from refact import traj_compressor
from refact.cli_app_switcher import start_app, exit_all_apps, push_app
from refact import cli_statusbar, cli_settings
from refact.lsp_runner import LSPServerRunner


lsp_runner: Optional[LSPServerRunner] = None
app: Optional[Application] = None


async def answer_question_in_arguments(settings, arg_question):
    cli_streaming.add_streaming_message(chat_client.Message(role="user", content=arg_question))
    cli_streaming.start_streaming()
    await the_chatting_loop(settings.model, max_auto_resubmit=4)
    cli_streaming.flush_response()


tips_of_the_day = '''
Ask anything: "does this project have SQL in it?"
Ask anything: "summarize README"
Refact Agent is essentially its tools, ask: "what tools do you have?"
'''.strip().split('\n')


async def welcome_message(settings: cli_settings.CmdlineArgs, tip: str):
    text = f"""
~/.cache/refact/cli.yaml                -- set up this program
~/.cache/refact/bring-your-own-key.yaml -- set up models you want to use
~/.cache/refact/integrations.yaml       -- set up github, jira, make, gdb, and other tools, including which actions require confirmation
~/.cache/refact/privacy.yaml            -- which files should never leave your computer
Project: {settings.project_path}
To exit, type 'exit' or Ctrl+D. {tip}.
"""
    cli_printing.print_formatted_text(FormattedText([
        (f"fg:#707070", text.strip()),
    ]))


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
        response = get_at_command_completion(lsp_runner.base_url(), text, position)
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
    if user_input.lower() in ["exit", "quit", "/exit", "/quit"]:
        exit_all_apps()
        return

    if cli_streaming.is_streaming():
        return

    if user_input.startswith("/"):  # commands
        args = user_input.split(" ")
        if args[0] == "/export":
            loop = asyncio.get_event_loop()
            loop.create_task(cli_export.think_of_good_filename_and_export(cli_streaming.streaming_messages))
            return
        elif args[0] == "/hint":
            if cli_streaming.streaming_messages[-1].role != "assistant":
                print_formatted_text(f"\nthe last message is not role=\"assistant\", you can't use /hint here")
                return
            cli_streaming.streaming_messages.pop()
            if cli_streaming.streaming_messages[-1].role == "cd_instruction":
                cli_streaming.streaming_messages.pop()
            cli_streaming.streaming_messages.append(chat_client.Message(
                role="cd_instruction",
                content=("💿 Hint: %s" % " ".join(args[1:])),
            ))
            user_input = ""
        elif args[0] == "/attach":
            error_messages = []
            for filename in glob.iglob(args[1]):
                try:
                    filename_ext = filename.split(".")[-1]
                    ext2type = {
                        "jpg": "image/jpeg",
                        "jpeg": "image/jpeg",
                        "png": "image/png",
                    }
                    if filename_ext not in ext2type:
                        raise RuntimeError(f"unknown extension '{filename_ext}'")
                    m_type = ext2type[filename_ext]
                    with open(filename, "rb") as f:
                        m_content = base64.b64encode(f.read()).decode("utf-8")
                    cli_streaming.streaming_messages.append(chat_client.Message(
                        role="user",
                        content=[{
                            "m_type": m_type,
                            "m_content": m_content,
                        }],
                    ))
                except Exception as e:
                    error_messages.append(f"can't attach file {filename}: {e}")
            if error_messages:
                cli_streaming.streaming_messages.append(chat_client.Message(
                    role="user",
                    content="\n".join(error_messages),
                ))
            user_input = ""
        else:
            print_formatted_text(f"\nchat> {user_input}")
            print_formatted_text(f"\nunknown command %s" % args[0])
            return

    elif user_input == "" and len(cli_streaming.streaming_messages) > 0:
        last_message = cli_streaming.streaming_messages[-1]
        if last_message.role == "assistant" and last_message.tool_calls is not None:
            pass   # re-submit tool calls
        else:
            return

    elif user_input.strip() == "":
        print_formatted_text("\n")
        return

    elif user_input.startswith("?"):
        label = user_input[1:]
        if open_label(label):
            push_app(inspect_app())
        else:
            print_formatted_text(f"\nchat> {user_input}")
            print_formatted_text(FormattedText([("#ff3333", f"label {label} not found")]))
        return

    if user_input.strip() != "":
        print_response(f"\nchat> {user_input}\n")
        cli_streaming.add_streaming_message(chat_client.Message(role="user", content=user_input))

    start_streaming()

    async def asyncfunc():
        await the_chatting_loop(cli_settings.args.model, cli_settings.args.chat_id, max_auto_resubmit=(1 if cli_settings.args.always_pause else 6))
        if len(cli_streaming.streaming_messages) == 0:
            return
        # cli_streaming.print_response("\n")  # flush_response inside
        cli_streaming.flush_response()
        last_message = cli_streaming.streaming_messages[-1]
        if last_message.role == "assistant" and last_message.tool_calls is not None:
            for tool_call in last_message.tool_calls:
                function = tool_call.function
                cli_printing.print_formatted_text(FormattedText([
                    (f"fg:#707070", f"🔨PAUSED {function.name}({function.arguments})\n")
                ]))
            cli_printing.print_formatted_text(FormattedText([
                (f"fg:#707070", f"tool calls paused because of max_auto_resubmit, press Enter to submit"),
            ]))

    loop = asyncio.get_event_loop()
    loop.create_task(asyncfunc())


async def chat_main():
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
    parser.add_argument('--xdebug', type=int, default=0, help="Connect to refact-lsp on the given port, as opposed to starting a new refact-lsp process")
    parser.add_argument('--always-pause', action='store_true', help="Pause even if the model tries to run tools, normally that's submitted automatically")
    parser.add_argument('--start-with', type=str, default=False, help="Start with messages in a .json file, the format is [msg, msg, ...]")
    parser.add_argument('--compressor', action='store_true', help="Compress trajectory that comes from reading --start-with and exit")
    parser.add_argument('--chat-id', type=str, default=None, help="Optional unique id of the chat")
    parser.add_argument('question', nargs=argparse.REMAINDER, help="You can continue your question in the command line after --")
    args_parsed = parser.parse_args(before_minus_minus)
    arg_question = " ".join(after_minus_minus)

    cli_settings.cli_yaml = cli_settings.load_cli_or_auto_configure()

    refact_args = [
        os.path.join(os.path.dirname(__file__), "bin", "refact-lsp"),
    ]
    if cli_settings.cli_yaml.address_url:
        refact_args.extend(["--address-url", cli_settings.cli_yaml.address_url])
    if cli_settings.cli_yaml.api_key:
        refact_args.extend(["--api-key", cli_settings.cli_yaml.api_key])
    if cli_settings.cli_yaml.insecure_ssl:
        refact_args.append("--insecure-ssl")
    if cli_settings.cli_yaml.basic_telemetry:
        refact_args.append("--basic-telemetry")
    if cli_settings.cli_yaml.experimental:
        refact_args.append("--experimental")
    if cli_settings.cli_yaml.ast:
        refact_args.extend(["--ast", "--ast-max-files", str(cli_settings.cli_yaml.ast_max_files)])
    if cli_settings.cli_yaml.vecdb:
        refact_args.extend(["--vecdb", "--vecdb-max-files", str(cli_settings.cli_yaml.vecdb_max_files)])
    if args_parsed.path_to_project:
        refact_args.extend(["--workspace-folder", args_parsed.path_to_project])
    lsp_runner = LSPServerRunner(
        refact_args,
        wait_for_ast_vecdb=False,
        refact_lsp_log=None,
        verbose=False
    )

    lsp_runner.set_xdebug(args_parsed.xdebug)
    chat_id = args_parsed.chat_id or ("cli-" + ''.join(random.choices('0123456789abcdef', k=10)))

    async with lsp_runner:
        caps = await cli_settings.fetch_caps(lsp_runner.base_url())
        cli_settings.args = cli_settings.CmdlineArgs(
            caps,
            model=args_parsed.model,
            path_to_project=args_parsed.path_to_project,
            always_pause=args_parsed.always_pause,
            chat_id=chat_id,
        )
        await actual_chat(lsp_runner, start_with=args_parsed.start_with, caps=caps, arg_question=arg_question, run_compressor=args_parsed.compressor)


async def actual_chat(
    lsp_runner_,
    *,
    caps: cli_settings.Caps,
    arg_question: str = "",
    run_compressor: bool = False,
    start_with: str,
):
    if start_with:
        with open(start_with, "r") as f:
            startwith = json.loads(f.read())
        for msg_j in startwith:
            cli_streaming.process_streaming_data(msg_j, None)
        cli_streaming.flush_response()
        cli_printing.print_formatted_text(FormattedText([
            (f"fg:#808080", "\n\n -- started with %d messages --\n" % len(cli_streaming.streaming_messages)),
        ]))

    global lsp_runner
    lsp_runner = lsp_runner_
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
        Window(content=get_entertainment_box(), dont_extend_height=True),
        ConditionalContainer(
            content=FloatContainer(content=vsplit, floats=[
                Float(xcursor=True, ycursor=True, content=CompletionsMenu())]
            ),
            filter=is_not_streaming_condition,
        ),
        Window(),
        cli_statusbar.StatusBar(),
    ])

    layout = Layout(hsplit)
    global app
    app = Application(key_bindings=kb, layout=layout)
    app.editing_mode = cli_settings.cli_yaml.get_editing_mode()

    if cli_settings.args.model not in caps.code_chat_models:
        known_models = list(caps.code_chat_models.keys())
        print(f"model {cli_settings.args.model} is unknown, pick one of {known_models}")
        return

    cli_statusbar.model_section = f"model {cli_settings.args.model} context {cli_settings.args.n_ctx()}"

    if run_compressor:
        await traj_compressor.trajectory_compressor(cli_streaming.streaming_messages)
        return

    await welcome_message(cli_settings.args, random.choice(tips_of_the_day))

    if arg_question:
        print(arg_question)
        await answer_question_in_arguments(cli_settings.args, arg_question)
        return

    asyncio.create_task(cli_statusbar.statusbar_background_task())
    await start_app(app)


def entrypoint():
    """
    `refact` in console runs this function.
    """
    asyncio.run(chat_main())


if __name__ in '__main__':
    asyncio.run(chat_main())
