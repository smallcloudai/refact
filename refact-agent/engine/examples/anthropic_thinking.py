import asyncio

from refact import chat_client


async def main():
    messages = [
        chat_client.Message(role="system", content="super system"),
    ]

    chat_id = "9999"
    tools_turn_on = {"tree", "cat"}
    tools = await chat_client.tools_fetch_and_filter(base_url="http://127.0.0.1:8001/v1", tools_turn_on=tools_turn_on)
    while True:
        user_content = input("User: ")
        messages.append(chat_client.Message(role="user", content=user_content))
        while True:
            assistant_choices = await chat_client.ask_using_http(
                base_url="http://127.0.0.1:8001/v1",
                messages=messages,
                n_answers=1,
                model_name="claude-3-7-sonnet",
                tools=tools,
                tool_choice="auto",
                stream=False,
                max_tokens=2048,
                verbose=False,
                chat_id=chat_id,
                boost_thinking=True,
            )
            new_messages = assistant_choices[0][len(messages):]
            has_tool_calls = False
            for m in new_messages:
                print(f"{m.role}: {m.content} ({len(m.thinking_blocks or [])} blocks, {len(m.tool_calls or [])} tools)")
                if m.tool_calls:
                    has_tool_calls = True
            messages.extend(new_messages)
            if not has_tool_calls:
                break


if __name__ == "__main__":
    asyncio.run(main())
