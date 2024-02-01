import requests, json, termcolor


my_prompt = """
You are a bot good at explaining the purpose for the given code.

STEPS:

In the [ORIGINAL_CODE_STEP] user will provide code surrounding the code snippet in question, and then the snippet itself will start with 🔥code and backquotes.

In the [PROVIDE_COMMANDS_STEP] you have to ask for an extra context to completely understand the 🔥code and it's role in the project.
Run several commands in a single message. Don't write any explanations on this step.
Write the number of commands you plan to issue as a first line of your response,
and then write all the commands.
Commands available:

🔍SEARCH <search query> to find more information in other source files in the project or documentation. It's good for looking up definitions and usage.

🔍FILE <path/file> to dump whole file text.

Ask for definitions of types used in the 🔥code.
Ask for usages of the class or function defined in the 🔥code.
Don't look up symbols you already have.

An examples of commands:

🔍SEARCH usages of function f

🔍SEARCH definition of Type2

🔍FILE repo1/test_file.cpp

In the [GENERATE_DOCUMENTATION_STEP] you have to generate an explanation of the 🔥code.
Answer questions "why it exists", "how does it fit into broader context". Don't explain line-by-line. Don't explain class data fields.
"""

to_explain = """pub struct DeltaDeltaChatStreamer {
    pub delta1: String,
    pub delta2: String,
    pub finished: bool,
    pub stop_list: Vec<String>,
    pub role: String,
}
"""

initial_messages = [
{"role": "system", "content": my_prompt},
{"role": "user", "content":
    "[ORIGINAL_CODE_STEP]\n" +
    "@file /home/user/.refact/tmp/unpacked-files/refact-lsp/src/scratchpads/chat_utils_deltadelta.rs\n" +
    "Why this 🔥code exists:\n```\n[CODE]```\n".replace("[CODE]", to_explain) +
    "[PROVIDE_COMMANDS_STEP]\n"},
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
    # data: {"choices":[{"delta":{"content":"The","role":"assistant"},"finish_reason":null,"index":0}],"created":1706779319.409,"model":"gpt-3.5-turbo"}
    # data: {"choices":[{"delta":{"content":" code","role":"assistant"},"finish_reason":null,"index":0}],"created":1706779319.409,"model":"gpt-3.5-turbo"}
    # Collect all delta/content from the response
    messages_back = []
    accum_content = ""
    accum_role = ""
    # print(response.text)
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


def rewrite_assistant_says_to_at_commands(ass):
    out = ""
    for s in ass.splitlines():
        s = s.strip()
        if not s:
            continue
        if s.startswith("🔍SEARCH"):
            out += "@workspace " + s[8:] + "\n"
        if s.startswith("🔍FILE"):
            out += "@file " + s[6:] + "\n"
    return out


def dialog_turn(messages):
    for msgdict in messages:
        print(termcolor.colored(msgdict["role"], "blue"))
        print(termcolor.colored(msgdict["content"], "green"))
    messages_back = ask_chat(messages)
    for msgdict in messages_back:
        print(termcolor.colored(msgdict["role"], "blue"))
        print(termcolor.colored(msgdict["content"], "red"))
    assistant_says = messages_back[-1]["content"]
    messages_without_last_user = messages[:-1]
    next_step_messages = messages_without_last_user + messages_back
    automated_new_user = rewrite_assistant_says_to_at_commands(assistant_says)
    if not automated_new_user:
        return next_step_messages, False
    automated_new_user += "[GENERATE_DOCUMENTATION_STEP]"
    next_step_messages.append({"role": "user", "content": automated_new_user})
    return next_step_messages, True


def do_all():
    messages = initial_messages.copy()
    for step in range(2):
        print("-"*40, "STEP%02d" % step, "-"*40)
        messages, need_automated_post = dialog_turn(messages)
        if not need_automated_post:
            break


do_all()
