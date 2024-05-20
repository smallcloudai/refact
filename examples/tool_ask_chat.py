import json
import requests


code_in_question = """
if __name__ == "__main__":
    class Toad(frog.Frog):
        def __init__(self, x, y, vx, vy):
            super().__init__(x, y, vx, vy)
            self.name = "Bob"
    toad = EuropeanCommonToad(100, 100, 200, -200)
    toad.jump(W, H)
    print(toad.name, toad.x, toad.y)
"""


messages = [
    ["system", "You are a coding assistant. Use your sense of humor"],
    ["user", "Explain what that code does\n```%s```" % code_in_question],
]


def parse(line: str):
    try:
        line = line.replace('data: ', '')
        return json.loads(line)
    except Exception:
        return None


def deserialize(value: str):
    try:
        return json.loads(value)
    except Exception:
        return None


def ask_chat(msgs, tool_use, endpoint: str = "http://127.0.0.1:8001/v1/chat/completions"):
    j = {
        "messages": msgs,
        "temperature": 0.6,
        "max_tokens": 512,
        "model": "gpt-4o",
        "stream": True,
        "tool_use": True if tool_use else False,
    }

    print(f"Tool use: {j.get('tool_use', False)}")
    response = requests.post(
        endpoint,
        json=j,
        timeout=60,
    )
    assert response.status_code == 200
    return response.text


def answer_plain_text(text: str) -> str:
    resp = [p for line in text.split("\n") if (p := parse(line))]
    resp = [c for p in resp if (c := p.get('choices', [{}])[0].get('delta', {}).get('content'))]
    return "".join(resp)


def ask():
    r1 = ask_chat(messages, True)
    r1_parsed = parse(r1)
    print(r1_parsed)
    messages[-1] = [*messages[-1], r1_parsed.get('tool_calls')]
    print(messages[-1][1])
    r2 = ask_chat(messages, False)
    # print(r2)
    print(answer_plain_text(r2))


if __name__ == "__main__":
    ask()