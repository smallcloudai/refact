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


def ask_chat(msgs, tool_choice, endpoint: str = "http://127.0.0.1:8001/v1/chat"):
    j = {
        "messages": msgs,
        "temperature": 0.6,
        "max_tokens": 512,
        "model": "gpt-4o",
        "stream": True,
        "tool_choice": tool_choice,
    }
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


def collect_tools(resp: str):
    tools = {}
    for l in [p for l in resp.split("\n") if (p := parse(l))]:
        ch0 = l.get("choices")[0]
        if not (tool_calls := ch0.get('delta').get("tool_calls")):
            continue
        f0 = tool_calls[0]
        if not tools.get(f0["index"]):
            tools[f0["index"]] = f0
        else:
            tools[f0["index"]]['function']["arguments"] += f0['function']["arguments"]

    return list(tools.values())


def ask():
    r1 = ask_chat(messages, "required")
    tools = collect_tools(r1)
    messages.append(
        ["assistant", "", tools]
    )
    r2 = ask_chat(messages, "none")
    print(r2)
    print(answer_plain_text(r2))


if __name__ == "__main__":
    ask()
