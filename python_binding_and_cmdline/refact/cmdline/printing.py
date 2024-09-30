from typing import List, Tuple
from prompt_toolkit import HTML
from prompt_toolkit.shortcuts import print_formatted_text
from prompt_toolkit.styles import Style
from prompt_toolkit.formatted_text import PygmentsTokens, FormattedText
from pygments.lexers import guess_lexer_for_filename, get_lexer_by_name, guess_lexer
import pygments
import shutil

from refact.cmdline import settings

Tokens = List[Tuple[str, str]]
Lines = List[Tokens]


def get_terminal_width() -> int:
    return shutil.get_terminal_size((80, 20))[0]


def split_newline_tokens(tokens: Tokens) -> Tokens:
    result = []
    for token in tokens:
        first = True
        for line in token[1].split("\n"):
            if not first:
                result.append((token[0], "\n"))
            result.append((token[0], line))
            if first:
                first = False
    return result


def wrap_tokens(tokens: Tokens, max_width: int) -> Lines:
    tokens: Tokens = split_newline_tokens(tokens)
    result = []
    current_line = []
    line_length = 0
    for token in tokens:
        token_len = len(token[1])
        if token_len + line_length > max_width:
            result.append(current_line)
            current_line = []
            line_length = 0
        while token_len > max_width:
            result.append((token[0], token[1][:max_width]))
            token = (token[0], token[1][max_width:])
            token_len = len(token[1])

        if token[1] == "\n":
            result.append(current_line)
            current_line = []
            line_length = 0
        elif token_len != 0:
            current_line.append(token)
            line_length += token_len
    return result


def to_tokens(text: str) -> Tokens:
    return [("", text)]


def tokens_len(tokens: Tokens) -> int:
    return sum([len(x[1]) for x in tokens])


def highlight_text_by_language(text: str, language: str) -> Tokens:
    try:
        lexer = get_lexer_by_name(language)
        tokens = list(pygments.lex(text, lexer=lexer))
        return PygmentsTokens(tokens).__pt_formatted_text__()
    except:
        lexer = guess_lexer(language)
        tokens = list(pygments.lex(text, lexer=lexer))
        return PygmentsTokens(tokens).__pt_formatted_text__()


def highlight_text(text: str, file_name: str) -> Tokens:
    lexer = guess_lexer_for_filename(file_name, text)
    tokens = list(pygments.lex(text, lexer=lexer))
    return PygmentsTokens(tokens).__pt_formatted_text__()


def set_tokens_background_color(tokens: Tokens, color: str) -> Tokens:
    return [(x[0] + f" bg:{color}", x[1]) for x in tokens]


def set_background_color(lines: Lines, color: str) -> Lines:
    terminal_width = get_terminal_width()
    return [
        set_tokens_background_color(line, color) + [(f"bg:{color}", " " * (terminal_width - tokens_len(line)))]
        for line in lines
    ]


def limit_lines(lines: Lines, max_height: int) -> Lines:
    if len(lines) > max_height:
        return lines[0:max_height - 1] + [to_tokens(f"... {len(lines) - max_height} lines hidden ...")]
    return lines


def print_lines(lines: Lines):
    for line in lines:
        print_formatted_text(FormattedText(line))


def print_file_name(file_name: str):
    nerd_font = settings.cli_yaml.nerd_font
    tab_color = "#3e4957"
    if nerd_font:
        print_formatted_text(FormattedText([
            (tab_color, " "),
            (f"bg:{tab_color}", f" {file_name} "),
            (tab_color, ""),
        ]))
    else:
        print_formatted_text(FormattedText([
            ("", " "),
            (f"bg:{tab_color}", f"  {file_name}  "),
        ]))


def print_file(content: str, file_name: str):
    bg_color = "#252b37"

    terminal_width = get_terminal_width()
    content = highlight_text(content, file_name)
    wrapped = wrap_tokens(content, terminal_width - 2)
    limited = limit_lines(wrapped, 15)
    colored = set_background_color(limited, bg_color)
    print_file_name(file_name)
    print_lines(colored)


def print_header(text: str, width: int) -> str:
    style = Style.from_dict({
        'block': 'bg:ansiwhite fg:ansiblack',
    })
    text_width = len(text)
    left = (width - text_width - 2) // 2
    right = width - text_width - 2 - left
    print_formatted_text(HTML("─" * left + "<block> " + text + " </block>" + "─" * right), style=style)
