import os
# os.environ["OPENAI_LOG"] = "debug"
# os.environ["OPENAI_LOG_JSON"] = "true"
import tool_ask_gpt as ask_gpt
import asyncio


code_in_question = """
if __name__ == "__main__":
    class Toad(frog.Frog):
        def __init__(self, x, y, vx, vy):
            super().__init__(x, y, vx, vy)
            self.name = "Bob"
    # toad = EuropeanCommonToad(100, 100, 200, -200)
    # toad.jump(W, H)
    # print(toad.name, toad.x, toad.y)
"""

messages = [
    ["system", "You are a coding assistant. Use your sense of humor. Before answering, use tool calls to fetch definitions of all the types and functions. Your first answer must consist of tool calls."],
    ["user", "Explain what that code does\n```%s```" % code_in_question],
]


async def do_all():
    tools = [
        {
            "type": "function",
            "function": {
                "name": "definition",
                "description": "Use abstract syntax tree to fetch the definition of a symbol, especially function, method, class, type alias.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "symbol": { "type": "string", "description": "Name to search, for example \"MyClass\", \"my_func\", \"MyClass::my_func\", use :: as a separator for paths"},
                    },
                    "required": ["symbol"],
                },
            },
        },
    ]

    ans = await ask_gpt.simple_ask_gpt(
        "step0", messages, 1, [], True,
        # model_name = "gpt-4-turbo",
        # model_name = "gpt-4o",
        # model_name = "gpt-3.5-turbo-1106",  # $1, multi call works
        model_name = "gpt-3.5-turbo-0125",    # $0.50, multi call doesn't work
        temperature=0.6,
        tools=tools,
        # tool_choice="required",
    )


asyncio.run(do_all())


#     response_message = response.choices[0].message
#     tool_calls = response_message.tool_calls
#     # Step 2: check if the model wanted to call a function
#     if tool_calls:
#         # Step 3: call the function
#         # Note: the JSON response may not always be valid; be sure to handle errors
#         available_functions = {
#             "get_current_weather": get_current_weather,
#         }  # only one function in this example, but you can have multiple
#         messages.append(response_message)  # extend conversation with assistant's reply

# {"role": "assistant", "tool_calls": [...]}
# {"role": "tool", "name": "get_current_weather", "content": "..."}

#         # Step 4: send the info for each function call and function response to the model
#         for tool_call in tool_calls:
#             function_name = tool_call.function.name
#             function_to_call = available_functions[function_name]
#             function_args = json.loads(tool_call.function.arguments)
#             function_response = function_to_call(
#                 location=function_args.get("location"),
#                 unit=function_args.get("unit"),
#             )
#             messages.append(
#                 {
#                     "tool_call_id": tool_call.id,
#                     "role": "tool",
#                     "name": function_name,
#                     "content": function_response,
#                 }
#             )  # extend conversation with function response
#         second_response = client.chat.completions.create(
#             model="gpt-3.5-turbo-0125",
#             messages=messages,
#         )  # get a new response from the model where it can see the function response
#         return second_response
# print(run_conversation())

