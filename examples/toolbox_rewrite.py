import requests
import termcolor
import chat_with_at_command


def rewrite_to_at_commands(messages):
    return requests.post(
        "http://127.0.0.1:8001/v1/rewrite-assistant-says-to-at-commands",
        json={
            "messages": messages,
        },
        headers={
            "Content-Type": "application/json",
            "Authorization": "Bearer XXX",
        },
        timeout=60,
    ).json()


toolbox_config = requests.get("http://127.0.0.1:8001/v1/toolbox-config", timeout=60).json()
toolbox_why_command = toolbox_config["commands"]["why"]
messages = toolbox_why_command["messages"][:]
for msgdict in messages:
    chat_with_at_command.msg_pretty_print(msgdict, normal_color="green")

messages.append(
	{"role": "assistant", "content": "Here be dragons!\n3\n🔍DEFINITION MyClass\n🔍FILE hello.cpp\n🔍SEARCH Bill Clinton\n"}
)

result = rewrite_to_at_commands(messages)
print(result)

assert "@file hello.cpp\n" in result["suggested_user_message"], "rewrites tool use into at-commands"
assert result["original_toolbox_command"] == "why", "rewrite picks this up from role='ignore' message"
assert "GENERATE_DOCUMENTATION_STEP" in result["suggested_user_message"], "rewrite detects PROVIDE_COMMANDS_STEP in the last user message"
assert result["auto_response"] == True, "if it can pick up GENERATE_DOCUMENTATION_STEP, then auto response is possible"
