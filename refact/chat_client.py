import aiohttp, os, termcolor, copy, json
from typing import Optional, List, Any, Tuple, Dict, Literal, Set
from pydantic import BaseModel


# Our version of chat protocol is very similar to OpenAI API, with these changes:
#   - `deterministic_messages`, refact-lsp returns this before the actual answer from the model (called choices),
#     and it also returns re-written user message to remove @-commands
# more?
# The ask_using_openai_client() function doesn't have this extensions, usesful to verify refact-lsp still can
# handle a client without these extensions.


class FunctionDict(BaseModel):
    arguments: str
    name: str


class ToolCallDict(BaseModel):
    id: str
    function: FunctionDict
    type: str


class Message(BaseModel):
    role: Literal["system", "assistant", "user", "tool", "diff", "context_file", "context_memory"]
    content: Optional[str] = None
    tool_calls: Optional[List[ToolCallDict]] = None
    finish_reason: str = ""
    tool_call_id: str = ""


def messages_to_dicts(
    messages: List[Message],
    verbose: bool,
    *,     # the rest is only there to print it
    tools: Optional[List[Dict[str, Any]]],
    temperature: float,
    model_name: str,
):
    listofdict = []
    if verbose:
        tools_namesonly = [x["function"]["name"] for x in tools] if tools else []
        print(termcolor.colored("------ call chat %s T=%0.2f tools=%s ------" % (model_name, temperature, tools_namesonly), "red"))
    for x in messages:
        if x.role in ["system", "user", "assistant", "tool", "context_file", "context_memory", "diff"]:
            listofdict.append({
                "role": x.role,
                "content": x.content,
                "tool_calls": [tc.dict() for tc in x.tool_calls] if x.tool_calls else None,
                "tool_call_id": x.tool_call_id
            })
        else:
            assert 0, x.role
        if not verbose:
            continue
        if x.role == "system":
            continue
        if x.role == "tool" and x.content is not None:
            print(termcolor.colored(x.role, "yellow"), "\n%s" % x.content.strip())
            continue
        tool_calls = ""
        if x.tool_calls is not None:
            tool_calls += "call"
            for tcall in x.tool_calls:
                tool_calls += " %s(%s)" % (tcall.function.name, tcall.function.arguments)
        print(termcolor.colored(x.role, "yellow"), str(x.content).replace("\n", "\\n"), termcolor.colored(tool_calls, "red"))
    return listofdict


def join_messages_and_choices(
    orig_messages: List[Message],
    deterministic_messages: List[Message],
    choices: List[Optional[Message]],
    verbose: bool
) -> List[List[Message]]:
    messages = list(orig_messages)
    while len(messages) > 0 and messages[-1].role == "user":
        messages.pop()
    messages.extend(deterministic_messages)
    if verbose:
        for msg in deterministic_messages:
            print("deterministic",
                termcolor.colored(str(msg.role), "yellow"),
                str(msg.content),
                termcolor.colored(str(msg.finish_reason), "red"))
    output = [copy.deepcopy(messages) for _ in range(len(choices))]
    for i, msg in enumerate(choices):
        if msg is None:
            continue
        msg: Message
        if verbose and isinstance(msg.content, str):
            print("result[%d]" % i,
                termcolor.colored(msg.content, "yellow"),
                termcolor.colored(msg.finish_reason, "red"))
        if verbose and isinstance(msg.tool_calls, list):
            for tcall in msg.tool_calls:
                print("result[%d]" % i,
                    termcolor.colored("%s(%s)" % (tcall.function.name, tcall.function.arguments), "red"),
                )
        if isinstance(msg.tool_calls, list) and len(msg.tool_calls) == 0:
            msg.tool_calls = None
        output[i].append(msg)
    return output


async def tools_fetch_and_filter(base_url: str, tools_turn_on: Optional[Set[str]]) -> Optional[List[Dict[str, Any]]]:
    async def get_tools():
        async with aiohttp.ClientSession() as session:
            async with session.get(base_url + "/tools", timeout=1) as response:
                assert response.status == 200
                return await response.json()
    tools = None
    tools = await get_tools()
    if tools_turn_on is not None:
        tools = [x for x in tools if x["type"] == "function" and x["function"]["name"] in tools_turn_on]
    return tools


class ChoiceDeltaCollector:
    def __init__(self, n_answers: int):
        self.n_answers = n_answers
        self.choices = [Message(role="assistant", content="") for _ in range(n_answers)]

    def add_deltas(self, j_choices: List[Dict[str, Any]]):
        assert len(j_choices) == self.n_answers
        for j_choice in j_choices:
            choice: Message = self.choices[j_choice["index"]]
            delta = j_choice["delta"]
            if (j_tool_calls := delta.get("tool_calls", None)) is not None:
                for plus_tool in j_tool_calls:
                    # {'function': {'arguments': '', 'name': 'definition'}, 'id': 'call_gek85Z8bjtjo2VnlrrDE89WP', 'index': 0, 'type': 'function'}
                    # {'function': {'arguments': '{"sy'}, 'index': 0}]
                    # {'function': {'arguments': '', 'name': 'definition'}, 'id': 'call_OVdofaKjMgWIu5z0mmuHiMou', 'index': 1, 'type': 'function'}
                    # {'function': {'arguments': '{"sy'}, 'index': 1}
                    tool_idx = plus_tool["index"]
                    assert 0 <= tool_idx < 100, f"oops tool_idx is {tool_idx}"
                    if choice.tool_calls is None:
                        choice.tool_calls = []
                    while len(choice.tool_calls) <= tool_idx:
                        choice.tool_calls.append(ToolCallDict(id="", function=FunctionDict(arguments="", name=""), type=""))
                    tool = choice.tool_calls[tool_idx]
                    if (i := plus_tool.get("id", None)) is not None and isinstance(i, str):
                        tool.id = i
                    if (t := plus_tool.get("type", None)) is not None and isinstance(t, str):
                        tool.type = t
                    if (function_plus := plus_tool.get("function", None)) is not None:
                        function_plus = plus_tool["function"]
                        if (n := function_plus.get("name", None)) is not None and isinstance(n, str):
                            tool.function.name += n
                        if (a := function_plus.get("arguments", None)) is not None and isinstance(a, str):
                            tool.function.arguments += a
            elif plus_content := delta.get("content"):
                # print("CONTENT", plus_content)
                choice.content += plus_content
            elif "finish_reason" in j_choice:
                choice.finish_reason = j_choice["finish_reason"]
            else:
                print("unrecognized delta", j_choice)


async def ask_using_http(
    base_url: str,
    messages: List[Message],
    n_answers: int,
    model_name: str,
    *,
    stop: List[str] = [],
    tools: Optional[List[Dict[str, Any]]] = None,
    tool_choice: Optional[str] = None,
    temperature: float = 0.6,
    stream: bool = False,
    verbose: bool = True,
    max_tokens: int = 1000,
    only_deterministic_messages: bool = False,
) -> List[List[Message]]:
    deterministic: List[Message] = []
    post_me = {
        "model": model_name,
        "n": n_answers,
        "messages": messages_to_dicts(messages, verbose, tools=tools, temperature=temperature, model_name=model_name),
        "temperature": temperature,
        "top_p": 0.95,
        "stop": stop,
        "stream": stream,
        "tools": tools,
        "tool_choice": tool_choice,
        "max_tokens": max_tokens,
        "only_deterministic_messages": only_deterministic_messages,
    }
    choices: List[Optional[Message]] = [None] * n_answers
    async with aiohttp.ClientSession() as session:
        async with session.post(base_url + "/chat", json=post_me) as response:
            assert response.status == 200
            if not stream:
                j = await response.json()
                deterministic = [Message(**x) for x in j.get("deterministic_messages", [])]
                j_choices = j["choices"]
                for i, ch in enumerate(j_choices):
                    index = ch["index"]
                    tool_calls = ch["message"].get("tool_calls", None)
                    msg = Message(
                        role=ch["message"]["role"],
                        content=ch["message"]["content"],
                        tool_calls=[ToolCallDict(**x) for x in tool_calls] if tool_calls is not None else None,
                        finish_reason=ch["finish_reason"],
                    )
                    choices[index] = msg
            else:
                choice_collector = ChoiceDeltaCollector(n_answers)
                async for line in response.content:
                    line_str = line.decode('utf-8').strip()
                    if not line_str:
                        continue
                    if not line_str.startswith("data: "):
                        print("unrecognized streaming data (1):", line_str)
                        continue
                    line_str = line_str[6:]
                    # print(">>>", line_str)
                    if line_str == "[DONE]":
                        break
                    j = json.loads(line_str)
                    if "choices" in j:
                        choice_collector.add_deltas(j["choices"])
                    elif "role" in j:
                        deterministic.append(Message(**j))
                    else:
                        print("unrecognized streaming data (2):", j)
                choices = [(None if not x.content else x) for x in choice_collector.choices]
    return join_messages_and_choices(messages, deterministic, choices, verbose)


async def ask_using_openai_client(
    base_url: str,
    messages: List[Message],
    n_answers: int,
    model_name: str,
    *,
    stop: List[str],
    tools: Optional[List[Dict[str, Any]]] = None,
    temperature: float = 0.6,
    verbose: bool = True,
    max_tokens: int = 1000,
) -> List[List[Message]]:
    import openai
    # os.environ["OPENAI_LOG"] = "debug"
    # os.environ["OPENAI_LOG_JSON"] = "true"
    aclient = openai.AsyncOpenAI(
        base_url=base_url,
        api_key=os.getenv("OPENAI_API_KEY"),
    )
    chat_completion = await aclient.chat.completions.create(
        model=model_name,
        n=n_answers,
        messages=messages_to_dicts(messages, verbose, tools=tools, temperature=temperature, model_name=model_name),
        temperature=temperature,
        top_p=0.95,
        stop=stop,
        stream=False,
        tools=tools,
        max_tokens=max_tokens
    )
    assert isinstance(chat_completion, openai.types.chat.chat_completion.ChatCompletion)
    # TODO: chat_completion.deterministic_messages
    deterministic: List[Message] = []
    choices: List[Optional[Message]] = [None] * len(chat_completion.choices)
    for i, ch in enumerate(chat_completion.choices):
        index = ch.index
        msg = Message(
            role=ch.message.role,
            content=ch.message.content,
            tool_calls=[ToolCallDict(**x.dict()) for x in ch.message.tool_calls] if ch.message.tool_calls is not None else None,
            finish_reason=ch.finish_reason
        )
        choices[index] = msg
    choices_not_none: List[Message] = [msg for msg in choices if msg is not None]
    return join_messages_and_choices(messages, deterministic, choices_not_none, verbose)


async def diff_apply(
    base_url: str,
    formatted_diff: List[Dict[str, Any]],
) -> List[List[Message]]:
    post_me = {
        "apply": [True] * len(formatted_diff),
        "chunks": formatted_diff,
    }
    async with aiohttp.ClientSession() as session:
        async with session.post(base_url + "/diff-apply", json=post_me) as response:
            if response.status != 200:
                raise Exception(f"unexpected response status {response.status}, response: {await response.text()}")
            return await response.json(content_type=None)
