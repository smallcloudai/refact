import json
from typing import Dict, List

from prompt_toolkit import print_formatted_text
from prompt_toolkit.layout.controls import FormattedTextControl
from prompt_toolkit.formatted_text import FormattedText
from prompt_toolkit.application import get_app

from refact.chat_client import Message, FunctionDict, ask_using_http, tools_fetch_and_filter
from refact.cmdline.printing import wrap_tokens, get_terminal_width
from refact.cmdline.printing import print_file, print_lines, highlight_text_by_language, set_background_color, print_file_name
from refact.cmdline.markdown import to_markdown
from refact.cmdline.inspect import create_label
from refact.cmdline import settings
from refact.cmdline import main
from prompt_toolkit.filters import Condition


response_text = ""
language_printing = None
response_box = FormattedTextControl(text=[])
streaming_messages: List[Message] = []
tool_calls: Dict[str, FunctionDict] = {}
streaming_toolcall: List[FunctionDict] = []
_is_streaming = False


def get_response_box():
    return response_box


def flush_response():
    global response_text
    global language_printing
    width = get_terminal_width()
    if response_text[:3] == "```":
        if language_printing is None:
            language_printing = response_text[3:]
            if language_printing.strip() != "":
                print_file_name(language_printing.strip())
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


def update_response_box():
    nerd_font = settings.cli_yaml.nerd_font
    response_box.text = [("", response_text)]
    for tool_call in tool_calls.values():
        function = tool_call["function"]
        response_box.text.append(("", f"\n🔨 {function['name']}({function['arguments']})"))
        if "context_files" in tool_call:
            context_files = tool_call["context_files"]
            if len(context_files) > 4:
                if nerd_font:
                    response_box.text.append(("", "\n    󰏢"))
                else:
                    response_box.text.append(("", "\n    📎"))
                response_box.text.append(("", f" <{len(context_files) - 4} more files>"))
            for context_file in context_files[-4:]:
                if nerd_font:
                    response_box.text.append(("", "\n    󰏢"))
                else:
                    response_box.text.append(("", "\n    📎"))
                response_box.text.append(("", f" {context_file}"))
        if "subchat_id" in tool_call:
            if nerd_font:
                response_box.text.append(("", f"\n     Subchat {tool_call['subchat_id']}"))
            else:
                response_box.text.append(("", f"\n    ⏳ Subchat {tool_call['subchat_id']}"))
    get_app().invalidate()


def print_response(to_print: str):
    global response_text
    for line in to_print.splitlines(True):
        response_text += line
        if line[-1] == "\n":
            flush_response()
    update_response_box()


def print_context_file(json_str: str):
    file = json.loads(json_str)[0]
    content = file["file_content"]
    file_name = file["file_name"]
    # line1 = file["line1"]
    # line2 = file["line2"]

    print_response("\n")
    flush_response()
    print_file(content, file_name)


def process_streaming_data(data):
    global streaming_messages
    global streaming_toolcall
    global tool_calls
    if "choices" in data:
        choices = data['choices']
        delta = choices[0]['delta']
        content = delta.get('content', None)

        # streaming tool calls
        if delta["tool_calls"] is not None:
            for tool_call in delta["tool_calls"]:
                id = tool_call["id"]
                index = tool_call["index"]
                if id is not None:
                    streaming_toolcall.append(tool_call)
                else:
                    streaming_toolcall[index]["function"]["arguments"] += tool_call["function"]["arguments"]
                    get_app().invalidate()

        if content is None:
            finish_reason = choices[0]['finish_reason']
            if finish_reason == 'stop':
                print_response("\n")
            if finish_reason == 'tool_calls':
                for tool_call in streaming_toolcall:
                    tool_calls[tool_call["id"]] = tool_call
                streaming_toolcall = []
                update_response_box()
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
        tool_call_id = data["tool_call_id"]
        print_response("\n")
        flush_response()
        label = create_label(content)
        if tool_call_id in tool_calls:
            tool_call = tool_calls.pop(tool_call_id)
            function = tool_call["function"]
            print_formatted_text(f"🔨 {function['name']}({function['arguments']}) ?{label}")
        else:
            print_formatted_text(f"🔨 <Unknown tool call {tool_call_id}> ?{label}")
        # print_lines(indented)

    elif "subchat_id" in data:
        # print_formatted_text(json.dumps(data, indent=2))

        subchat_id = data["subchat_id"]
        tool_call_id = data["tool_call_id"]
        if tool_call_id not in tool_calls:
            return

        tool_call = tool_calls[tool_call_id]
        tool_call["subchat_id"] = subchat_id

        add_message = data["add_message"]
        role = add_message["role"]
        content = add_message["content"]
        if role == "context_file":
            if "context_files" not in tool_call:
                tool_call["context_files"] = []
            content = json.loads(content)
            for file in content:
                tool_call["context_files"].append(file["file_name"])

        update_response_box()
        pass

    else:
        print_response("unknown streaming data:\n%s" % data)


async def ask_chat(model):
    global streaming_messages
    global _is_streaming

    N = 1
    for step_n in range(4):
        def callback(data):
            if _is_streaming:
                process_streaming_data(data)

        messages = list(streaming_messages)
        tools = await tools_fetch_and_filter(base_url=main.lsp.base_url(), tools_turn_on=None)
        new_messages = await ask_using_http(
            main.lsp.base_url(),
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

        if not _is_streaming:
            break
        if not streaming_messages[-1].tool_calls:
            break
    _is_streaming = False


def add_streaming_message(message: Message):
    streaming_messages.append(message)


@Condition
def is_not_streaming_condition():
    return not _is_streaming


def stop_streaming():
    global _is_streaming
    _is_streaming = False


def start_streaming():
    global _is_streaming
    _is_streaming = True


def is_streaming():
    return _is_streaming
