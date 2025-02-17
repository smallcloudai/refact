from refact.cli_printing import Tokens
from refact import cli_settings


gray = "#252b37"
green = "#6ac496"


def is_special_boundary(char: str) -> bool:
    return char in "*_[](){}:.,;!?-"


def is_word_boundary(text: str, i: int, after: bool = False, length: int = 1) -> bool:
    if after:
        return i + length >= len(text) or text[i + length].isspace() or is_special_boundary(text[i + length])
    else:
        return i - length < 0 or text[i - length].isspace() or is_special_boundary(text[i - length])


def is_beginning_of_line(text: str, i: int) -> bool:
    return i == 0 or text[i - 1] == "\n"


def to_markdown(text: str, width: int) -> Tokens:
    result = []
    last = -1
    i = 0

    is_bold = False
    is_italic = False
    is_inline_code = False
    header_level = 0

    def get_format():
        res = []
        if is_bold:
            res.append("bold")
        if is_italic:
            res.append("italic")
        if is_inline_code:
            res.append(f"bg:{gray}")
        if header_level > 0:
            res.append("reverse bold")
        return " ".join(res)

    while i < len(text):

        # `text`
        if text[i] == "`" and text[i + 1] != "`":
            result.append((get_format(), text[last + 1:i]))
            if header_level == 0:
                result.append((f"bg:{gray}", " "))
            last = i
            i += 1
            is_inline_code = True
            while i < len(text) and text[i] != "`":
                i += 1
            result.append((get_format(), text[last + 1:i]))
            if header_level == 0:
                result.append((f"bg:{gray}", " "))
            is_inline_code = False
            last = i

        # skip all backticks
        elif text[i] == "`":
            while i < len(text) and text[i] == "`":
                i += 1

        if not (i < len(text)):
            break

        # ### headers
        if text[i] == "#":
            result.append((get_format(), text[last + 1:i]))
            count = 0
            while i < len(text) and text[i] == "#":
                count += 1
                i += 1
            last = i - 1
            header_level = count

        # end of headers
        elif text[i] == "\n":
            if header_level > 0:
                result.append((get_format(), text[last + 1:i] + " \n"))
                header_level = 0
                last = i

        # *italic text*
        elif text[i] == "*" and text[i + 1] != "*" and is_word_boundary(text, i, is_italic):
            result.append((get_format(), text[last + 1:i]))
            last = i
            is_italic = not is_italic

        # _italic text_
        elif text[i] == "_" and text[i + 1] != "_" and is_word_boundary(text, i, is_italic):
            result.append((get_format(), text[last + 1:i]))
            last = i
            is_italic = not is_italic

        # **bold text**
        elif text[i:i + 2] == "**" and is_word_boundary(text, i, is_bold, 2):
            result.append((get_format(), text[last + 1:i]))
            i += 1
            last = i
            is_bold = not is_bold

        # __bold text__
        elif text[i:i + 2] == "__" and is_word_boundary(text, i, is_bold, 2):
            result.append((get_format(), text[last + 1:i]))
            i += 1
            last = i
            is_bold = not is_bold

        i += 1

    result.append(("", text[last + 1:]))
    return result
