import requests, json, termcolor


initial_messages = [
{"role": "user", "content":
    # "@workspace definition of DeltaDeltaChatStreamer\n" +
    #"@file /home/user/.refact/tmp/unpacked-files/refact-lsp/src/scratchpads/chat_utils_deltadelta.rs\n" +
    "@file chat_utils_deltadelta.rs\n" +
    # "@definition DeltaDeltaChatStreamer\n" +
    "summarize the code in one paragraph"
},
]

def ask_chat(messages):
    response = requests.post(
        "http://127.0.0.1:8001/v1/chat",
        json={
            "messages": messages,
            "temperature": 0.1,
            "max_tokens": 300,
            "model": "gpt-3.5-turbo",
        },
        headers={
            "Content-Type": "application/json",
            "Authorization": "Bearer XXX",
        },
        timeout=60,
    )
    messages_back = []
    accum_content = ""
    accum_role = ""
    for x in response.text.splitlines():
        if not x.strip():
            continue
        if not x.startswith("data: "):
            print(x)
            print("ERROR: unexpected response format")
            continue
        if x[6:].startswith("[DONE]"):
            break
        j = json.loads(x[6:])
        if "choices" in j:
            # streaming
            choice0 = j["choices"][0]
            accum_role = choice0["delta"]["role"]
            accum_content += choice0["delta"]["content"]
        else:
            # content/role without streaming, replacing the last user message
            messages_back.append({"role": j["role"], "content": j["content"]})
    if accum_role:
        messages_back.append({"role": accum_role, "content": accum_content})
    return messages_back


def example_single_response():
    messages_back = ask_chat(initial_messages)
    for msgdict in messages_back:
        msg_pretty_print(msgdict, normal_color="white")


def msg_pretty_print(msgdict, normal_color="white"):
    print(termcolor.colored(msgdict["role"], "blue"))
    if msgdict["role"] == "context_file":
        try:
            for x in json.loads(msgdict["content"]):
                print("%s:%i-%i" % (x["file_name"], x["line1"], x["line2"]))
        except json.decoder.JSONDecodeError:
            print(msgdict["content"])
    else:
        print(termcolor.colored(msgdict["content"], normal_color))


if __name__ == "__main__":
    example_single_response()

