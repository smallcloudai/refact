import aiohttp, os, termcolor, copy
from typing import Optional, List, Any, Tuple, Dict, Literal, Set
from pydantic import BaseModel


# Our version of chat protocol is very similar to OpenAI API, with these changes:
# - deterministic_messages, refact-lsp returns this before the actual answer from the model (called choices),
#   and it also returns re-written user message to remove @-commands


class FunctionDict(BaseModel):
    arguments: str
    name: str


class ToolCallDict(BaseModel):
    id: str
    function: FunctionDict
    type: str


class Message(BaseModel):
    role: Literal["system", "assistant", "user", "tool", "context_file"]
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
        if x.role in ["system", "user", "assistant", "tool", "context_file"]:
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
        if x.role == "tool":
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
    choices: List[Message],
    verbose: bool
):
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
        if verbose and isinstance(msg.content, str):
            print("result[%d]" % i,
                termcolor.colored(msg.content, "yellow"),
                termcolor.colored(msg.finish_reason, "red"))
        if verbose and isinstance(msg.tool_calls, list):
            for tcall in msg.tool_calls:
                print("result[%d]" % i,
                    termcolor.colored("%s(%s)" % (tcall.function.name, tcall.function.arguments), "red"),
                )
        output[i].append(msg)
    return output


async def tools_fetch_and_filter(base_url: str, tools_turn_on: Set[str]) -> Optional[List[Dict[str, Any]]]:
    async def get_tools():
        async with aiohttp.ClientSession() as session:
            async with session.get(base_url + "/at-tools-available", timeout=1) as response:
                assert response.status == 200
                return await response.json()
    tools = None
    if tools_turn_on:
        tools = await get_tools()
        tools = [x for x in tools if x["type"] == "function" and x["function"]["name"] in tools_turn_on]
    return tools


async def ask_using_http(
    base_url: str,
    messages: List[Tuple[str, str]],
    stop: List[str],
    verbose: bool,
    n_answers: int,
    model_name: str,
    *,
    tools: Optional[List[Dict[str, Any]]] = None,
    temperature: float = 0.6,
) -> List[Tuple[str, str]]:
    async with aiohttp.ClientSession() as session:
        post_me = {
            "model": model_name,
            "n": n_answers,
            "messages": messages_to_dicts(messages, verbose, tools=tools, temperature=temperature, model_name=model_name),
            "temperature": temperature,
            "top_p": 0.95,
            "stop": stop,
            "stream": False,
            "tools": tools,
        }
        async with session.post(base_url + "/chat", json=post_me) as response:
            assert response.status == 200
            j = await response.json()
    deterministic = [Message(**x) for x in j.get("deterministic_messages", [])]
    j_choices = j["choices"]
    choices: List[Optional[Message]] = [None] * len(j_choices)
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
    choices_not_none: List[Message] = [msg for msg in choices if msg is not None]
    return join_messages_and_choices(messages, deterministic, choices_not_none, verbose)


async def ask_using_openai_client(
    base_url: str,
    messages: List[Tuple[str, str]],
    stop: List[str],
    verbose: bool,
    n_answers: int,
    model_name: str,
    *,
    tools: Optional[List[Dict[str, Any]]] = None,
    temperature: float = 0.6,
) -> List[Tuple[str, str]]:
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
    )
    assert isinstance(chat_completion, openai.types.chat.chat_completion.ChatCompletion)
    print(chat_completion)
    # quit()
    # TODO: chat_completion.deterministic_messages
    deterministic = []
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
