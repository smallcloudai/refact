from typing import Dict, List, Any

from prompt_toolkit.layout.layout import Layout
from prompt_toolkit.layout import HSplit
from prompt_toolkit.application import Application
from prompt_toolkit.key_binding import KeyBindings
from prompt_toolkit.widgets import TextArea
from prompt_toolkit.clipboard.pyperclip import PyperclipClipboard
from refact import cli_statusbar, cli_settings
from refact.cli_app_switcher import exit_all_apps, pop_app


next_label_i = 0
labels: Dict[str, str] = {}


kb = KeyBindings()


@kb.add('c-d')
def exit_(event):
    exit_all_apps()


@kb.add('c-q')
@kb.add('q', eager=True)
def pop_(event):
    pop_app()


@kb.add('c-c', eager=True)
@kb.add('y', eager=True)
@kb.add('c', eager=True)
def copy_(event):
    data = text_area.buffer.copy_selection()
    event.app.clipboard.set_data(data)


def convert_to_base_x(n: int, x: int) -> List[int]:
    if n == 0:
        return [0]
    res = []
    while n > 0:
        res.append(n % x)
        n //= x
    return res[::-1]


def generate_label() -> str:
    global next_label_i
    i = next_label_i
    next_label_i += 1
    alphabet = 'abcdefghijklmnopqrstuvwxyz'
    base_26 = convert_to_base_x(i, 26)
    return "".join([alphabet[x] for x in base_26])


def create_label(value: Any) -> str:
    global labels
    new_label = generate_label()
    labels[new_label] = value
    return new_label


def line_number(line: int, wrap_count: int):
    return [("bg:#252b37", f"{line+1:>4} ")]

# TODO: highlight
# def print_file(content_: str, file_name: str):
#     bg_color = "#252b37"
#     terminal_width = get_terminal_width()
#     content = highlight_text(content_, file_name)
#     wrapped = wrap_tokens(content, terminal_width - 2)
#     limited = limit_lines(wrapped, 15)
#     colored = set_background_color(limited, bg_color)
#     print_file_name(file_name)
#     print_lines(colored)

text_area = TextArea(get_line_prefix=line_number, wrap_lines=True)
hsplit = HSplit([
    text_area,
    cli_statusbar.StatusBar(),
])
layout = Layout(hsplit)
_inspect_app: Application = Application(
    layout=layout,
    full_screen=True,
    key_bindings=kb,
    mouse_support=True,
    clipboard=PyperclipClipboard(),
)


def always_true():
    return True


def open_label(label: str) -> bool:
    if label in labels:
        text_area.read_only = False
        text_area.text = str(labels[label])
        text_area.read_only = True
        return True
    return False


def inspect_app() -> Application:
    assert cli_settings.cli_yaml is not None
    _inspect_app.editing_mode = cli_settings.cli_yaml.get_editing_mode()
    return _inspect_app
