import termcolor, requests
import chat_with_at_command


to_explain = """pub struct DeltaDeltaChatStreamer {
    pub delta1: String,
    pub delta2: String,
    pub finished: bool,
    pub stop_list: Vec<String>,
    pub role: String,
}
"""


def dialog_turn(messages):
    for msgdict in messages:
        chat_with_at_command.msg_pretty_print(msgdict, normal_color="green")
    messages_back = chat_with_at_command.ask_chat(messages)
    for msgdict in messages_back:
        chat_with_at_command.msg_pretty_print(msgdict, normal_color="white")

    rewrite_dict = requests.post(
        "http://127.0.0.1:8001/v1/rewrite-assistant-says-to-at-commands",
        json={
            "messages": messages_back,
        },
        headers={
            "Content-Type": "application/json",
            "Authorization": "Bearer XXX",
        },
        timeout=60,
    ).json()
    print(rewrite_dict)
    messages_without_last_user = messages[:-1]
    next_step_messages = messages_without_last_user + messages_back
    if rewrite_dict["suggested_user_message"]:
        next_step_messages.append({"role": "user", "content": rewrite_dict["suggested_user_message"]})
        return next_step_messages, True
    return next_step_messages, False


def do_all():
    toolbox_config = requests.get("http://127.0.0.1:8001/v1/toolbox-config", timeout=60).json()
    toolbox_why_command = toolbox_config["commands"]["why"]
    messages = toolbox_why_command["messages"][:]

    for msgdict in messages:
        msgdict["content"] = msgdict["content"] \
            .replace("%CODE_AROUND_CURSOR_JSON%", "") \
            .replace("%CODE_SELECTION%", to_explain)

    for step in range(2):
        print("-"*40, "STEP%02d" % step, "-"*40)
        messages, need_automated_post = dialog_turn(messages)
        if not need_automated_post:
            break


do_all()

