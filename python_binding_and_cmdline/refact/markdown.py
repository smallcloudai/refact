from refact.printing import Tokens, Lines

gray = "#252b37"

def to_markdown(text: str, width: int) -> Tokens:
    result = []
    last = -1
    i = 0
    while i < len(text):

        # `text`
        if text[i] == "`":
            result.append(("", text[last + 1:i]))
            last = i
            i += 1
            while i < len(text) and text[i]!= "`":
                i += 1
            result.append((gray, ""))
            result.append((f"italic bg:{gray}", text[last + 1:i]))
            result.append((gray, ""))
            last = i

        i += 1

    result.append(("", text[last + 1:]))
    return result

