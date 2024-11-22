import json
from typing import Dict, DefaultDict, List, Any, Optional
from collections import defaultdict

from prompt_toolkit import print_formatted_text
from prompt_toolkit.layout.controls import FormattedTextControl
from prompt_toolkit.formatted_text import FormattedText
from prompt_toolkit.application import get_app
from prompt_toolkit.filters import Condition

from refact.cli_printing import wrap_tokens, get_terminal_width, print_lines, highlight_text_by_language, set_background_color, print_file_name
from refact import cli_printing
from refact.cli_markdown import to_markdown
from refact.cli_inspect import create_label
from refact import cli_settings
from refact import cli_main
from refact import chat_client

STREAM = True


response_text = ""
language_printing = None
entertainment_box = FormattedTextControl(text=[])
streaming_messages: List[chat_client.Message] = []
tool_calls: Dict[str, chat_client.ToolCallDict] = {}
subchat_stuff: DefaultDict[str, Any] = defaultdict(dict)
streaming_toolcall: List[chat_client.ToolCallDict] = []
_is_streaming = False


def get_entertainment_box():
    return entertainment_box


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

    entertainment_box.text = []
    response_text = ""


def update_entertainment_box():
    entertainment_box.text = [("", response_text)]


for index, tool_call in enumerate(streaming_toolcall):
    # 🤔 🔨
    for index, tool_call in enumerate(streaming_toolcall):
        entertainment_box.text.append(("", f"\n🤔 {tool_call['function']['name']}({tool_call['function']['arguments']})"))
    for tool_call_id, subchat_in_tool in subchat_stuff.items():
        if "context_files" in subchat_in_tool:
            context_files = subchat_in_tool["context_files"]
            if len(context_files) > 4:
                entertainment_box.text.append(("", "\n    📎"))
                entertainment_box.text.append(("", f" <{len(context_files) - 4} more files>"))
            for context_file in context_files[-4:]:
                entertainment_box.text.append(("", "\n    📎"))
                entertainment_box.text.append(("", f" {context_file}"))
        if "subchat_id" in subchat_in_tool:
            entertainment_box.text.append(("", f"\n    ⏳ Subchat {subchat_in_tool['subchat_id']}"))
    get_app().invalidate()


def print_response(to_print: str):
    global response_text
    for line in to_print.splitlines(True):
        response_text += line
        if line[-1] == "\n":
            flush_response()
    update_entertainment_box()


def process_streaming_data(data: Dict[str, Any], deltas_collector: Optional[chat_client.ChoiceDeltaCollector]):
    global streaming_messages
    global streaming_toolcall
    global tool_calls
    term_width = get_terminal_width()

    if "choices" in data:
        assert deltas_collector is not None
        if not data.get("choices") and data.get("usage"):
            return
        choices = data['choices']
        delta = choices[0]['delta']
        delta_content = delta.get('content', None)

        if delta.get("tool_calls"):
            for tool_call in delta["tool_calls"]:
                assert deltas_collector.choices[0].tool_calls is not None
                streaming_toolcall = list(deltas_collector.choices[0].tool_calls)
                update_entertainment_box()
        finish_reason = choices[0]['finish_reason']
        if finish_reason == "stop":
            print_response("\n")
        if finish_reason == "tool_calls":
            for tool_call in streaming_toolcall:
                assert isinstance(tool_call, chat_client.ToolCallDict)
                tool_calls[tool_call.id] = tool_call
            update_entertainment_box()
        if delta_content is not None:
            print_response(delta_content)
        else:
            update_entertainment_box()

    elif ("role" in data) or isinstance(data, chat_client.Message):
        streaming_toolcall.clear()
        subchat_stuff.clear()
        update_entertainment_box()

        if isinstance(data, chat_client.Message):
            msg = data
        else:
            msg = chat_client.Message.model_validate(data)

        replace_last_user = False
        if msg.role == "user":
            if len(streaming_messages) > 0:
                if streaming_messages[-1].role == "user":
                    replace_last_user = True
        if replace_last_user:
            streaming_messages[-1] = msg
        else:
            streaming_messages.append(msg)

        if msg.role in ["context_file"]:
            context_file = json.loads(msg.content)
            for fdict in context_file:
                file_content = fdict["file_content"]
                file_name = fdict["file_name"]
                line1, line2 = fdict["line1"], fdict["line2"]
                attach = "📎 %s:%d-%d " % (file_name, line1, line2)
                while len(attach) < term_width - 10:
                    attach += "·"
                label = create_label(file_content)
                # don't print file_content, user can use label to see it
                print_formatted_text(f"{attach} ?{label}")
            print_response("\n")
            flush_response()

        elif msg.role in ["plain_text", "cd_instruction", "user"]:
            if replace_last_user:
                return
            print_response("\n")
            if isinstance(msg.content, str):
                print_response(msg.content.strip())
            elif isinstance(msg.content, list):
                collected_responses = ["multimodal["]
                for content in msg.content:
                    if content.get("m_type") == "text":
                        collected_responses.append("text=%r" % content.get("m_content"))
                    else:
                        collected_responses.append("not_text=%r" % content.get("m_type"))
                collected_responses = ["]"]
                print_response("\n".join(collected_responses))
            else:
                print_response("content is None, not normal\n")
            print_response("\n")

        elif msg.role in ["assistant"]:
            if msg.content is not None:
                if not STREAM:
                    print_response("\n" + msg.content.strip() + "\n")
            if msg.tool_calls is not None:
                for tool_call in msg.tool_calls:
                    assert isinstance(tool_call, chat_client.ToolCallDict)
                    tool_calls[tool_call.id] = tool_call

        elif msg.role in ["tool", "diff"]:
            print_response("\n")
            flush_response()
            tool_callout = ""
            if msg.tool_call_id in tool_calls:
                tool_call: chat_client.ToolCallDict = tool_calls.pop(msg.tool_call_id)
                function = tool_call.function
                tool_callout = f"🔨 {function.name}({function.arguments}) "
                # don't print content, user can use label to see it
            else:
                tool_callout = f"🔨 Unknown tool call {repr(msg.tool_call_id)} "
            while len(tool_callout) < term_width - 10:
                tool_callout += "·"
            label = create_label(msg.content)
            print_formatted_text(f"{tool_callout} ?{label}")

        else:
            assert 0, "unknown role=%s" % msg.role

    elif "subchat_id" in data:
        subchat_id = data["subchat_id"]
        subchat_in_tool = subchat_stuff[data["tool_call_id"]]
        subchat_in_tool["subchat_id"] = subchat_id

        add_message = data["add_message"]
        role = add_message["role"]
        content = add_message["content"]
        if role == "context_file":
            if "context_files" not in subchat_in_tool:
                subchat_in_tool["context_files"] = []
            content = json.loads(content)
            for file in content:
                subchat_in_tool["context_files"].append(file["file_name"])

        update_entertainment_box()

    else:
        print_response("unknown streaming data:\n%s" % data)


async def the_chatting_loop(model, chat_id, max_auto_resubmit):
    global streaming_messages
    global _is_streaming

    roles_str = " ".join(["%s/%d" % (msg.role, len(msg.content or "")) for msg in streaming_messages]) + " -> model"
    cli_printing.print_formatted_text(FormattedText([
        ("fg:#808080", "\n➤ %s" % roles_str),
    ]))

    # print_response("\n%d messages -> model" % len(streaming_messages))

    N = 1
    for step_n in range(max_auto_resubmit):
        def callback(data, deltas_collector):
            if _is_streaming:
                process_streaming_data(data, deltas_collector)

        messages = list(streaming_messages)
        tools = await chat_client.tools_fetch_and_filter(base_url=cli_main.lsp_runner.base_url(), tools_turn_on=None)
        choices = await chat_client.ask_using_http(
            cli_main.lsp_runner.base_url(),
            messages,
            N,
            model,
            tools=tools,
            verbose=False,
            temperature=0.3,
            stream=STREAM,
            max_tokens=2048,
            only_deterministic_messages=False,
            callback=callback,
            chat_id=chat_id,
        )
        streaming_messages = choices[0]

        if not _is_streaming:
            break
        if not streaming_messages[-1].tool_calls:
            break
    _is_streaming = False


def add_streaming_message(message: chat_client.Message):
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
