from __future__ import annotations
import uuid
import tabulate
import aiohttp
import os
import termcolor
import copy
import json
import time
from typing import Optional, List, Any, Tuple, DefaultDict, Dict, Literal, Set, Callable
import collections

from pydantic import BaseModel, ConfigDict
from rich.console import Console
from rich.markdown import Markdown


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


class Usage(BaseModel):
    prompt_tokens: int
    completion_tokens: int


class Message(BaseModel):
    role: Literal["system", "assistant", "user", "tool", "context_file", "diff", "plain_text", "cd_instruction"]
    content: Optional[str] = None
    tool_calls: Optional[List[ToolCallDict]] = None
    finish_reason: str = ""
    tool_call_id: str = ""
    usage: Optional[Usage] = None
    subchats: Optional[DefaultDict[str, List[Message]]] = None
    model_config = ConfigDict(exclude_none=True)


def messages_to_dicts(
    messages: List[Message],
    verbose: bool,
    *,     # the rest is only there to print it
    tools: Optional[List[Dict[str, Any]]],
    temperature: float,
    model_name: str,
) -> Tuple[List[Dict[str, Any]], str]:
    listofdict = []
    log = ""
    tools_namesonly = [x["function"]["name"] for x in tools] if tools else []
    log += termcolor.colored("------ call chat %s T=%0.2f tools=%s ------\n" % (model_name, temperature, tools_namesonly), "red")
    for x in messages:
        if x.role in ["system", "user", "assistant", "tool", "context_file", "diff", "plain_text", "cd_instruction"]:
            listofdict.append({
                "role": x.role,
                "content": x.content,
                "tool_calls": [tc.dict() for tc in x.tool_calls] if x.tool_calls else None,
                "tool_call_id": x.tool_call_id
            })
        else:
            assert 0, x.role
        if x.role == "system":
            continue
        if x.role == "tool" and x.content is not None:
            log += termcolor.colored(x.role, "yellow") + " " + \
                "\n%s" % termcolor.colored(x.content.strip(), "magenta") + "\n"
            continue
        tool_calls = ""
        if x.tool_calls is not None:
            tool_calls += "call"
            for tcall in x.tool_calls:
                tool_calls += " %s(%s)" % (tcall.function.name, tcall.function.arguments)
        # log += termcolor.colored(x.role, "yellow") + " " + str(x.content).replace("\n", "\\n") + " " + termcolor.colored(tool_calls, "red")
        log += termcolor.colored(x.role, "yellow") + " " + str(x.content) + " " + termcolor.colored(tool_calls, "red") + "\n"
    if verbose:
        print(log)
    return listofdict, log


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
    msg: Optional[Message]
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
        if verbose and isinstance(msg.content, str):
            print("result[%d]" % i,
                  termcolor.colored(msg.content, "yellow"),
                  termcolor.colored(msg.finish_reason, "red"))
        if verbose and isinstance(msg.tool_calls, list):
            for tcall in msg.tool_calls:
                print("result[%d]" % i, termcolor.colored("%s(%s)" % (tcall.function.name, tcall.function.arguments), "red"))
        if isinstance(msg.tool_calls, list) and len(msg.tool_calls) == 0:
            msg.tool_calls = None
        output[i].append(msg)
    return output


async def tools_fetch_and_filter(base_url: str, tools_turn_on: Optional[Set[str]]) -> Optional[List[Dict[str, Any]]]:
    async def get_tools():
        async with aiohttp.ClientSession() as session:
            async with session.get(base_url + "/tools", timeout=1) as response:
                text = await response.text()
                assert response.status == 200, f"unable to fetch tools: {response.status}, Text:\n{text}"
                return json.loads(text)
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
            j_index = j_choice["index"]
            if j_index < 0 or j_index >= self.n_answers:
                raise ValueError(f"add_deltas(): invalid choice index {j_index} for choices")
            choice: Message = self.choices[j_index]
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
    postprocess_parameters: Optional[Dict[str, Any]] = None,
    callback: Optional[Callable] = None,
) -> List[List[Message]]:
    deterministic: List[Message] = []
    subchats: DefaultDict[str, List[Message]] = collections.defaultdict(list)
    post_me = {
        "model": model_name,
        "n": n_answers,
        "messages": messages_to_dicts(messages, verbose, tools=tools, temperature=temperature, model_name=model_name)[0],
        "temperature": temperature,
        "top_p": 0.95,
        "stop": stop,
        "stream": stream,
        "tools": tools,
        "tool_choice": tool_choice,
        "max_tokens": max_tokens,
        "only_deterministic_messages": only_deterministic_messages,
    }
    if postprocess_parameters is not None:
        post_me["postprocess_parameters"] = postprocess_parameters
    choices: List[Optional[Message]] = [None] * n_answers
    async with aiohttp.ClientSession() as session:
        async with session.post(base_url + "/chat", json=post_me) as response:
            if not stream:
                text = await response.text()
                assert response.status == 200, f"/chat call failed: {response.status}\ntext: {text}"
                j = json.loads(text)
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
                        # NOTE: backend should send usage for each choice
                        usage=j.get("usage") if i == 0 else None,
                    )
                    choices[index] = msg
            else:
                choice_collector = ChoiceDeltaCollector(n_answers)
                buffer = b""
                async for data, end_of_http_chunk in response.content.iter_chunks():
                    buffer += data
                    if not end_of_http_chunk:
                        continue
                    line_str = buffer.decode('utf-8').strip()
                    buffer = b""
                    if not line_str:
                        continue
                    if not line_str.startswith("data: "):
                        print("unrecognized streaming data (1):", line_str)
                        continue
                    line_str = line_str[6:]
                    if line_str == "[DONE]":
                        break
                    j = json.loads(line_str)
                    # print(">>>", line_str)
                    if callback is not None:
                        callback(j)
                    if "choices" in j:
                        choice_collector.add_deltas(j["choices"])
                    elif "role" in j:
                        deterministic.append(Message(**j))
                    elif "subchat_id" in j:
                        map_key = j["tool_call_id"] + "__" + j["subchat_id"]
                        subchats[map_key].append(Message(**j["add_message"]))
                    else:
                        print("unrecognized streaming data (2):", j)
                end_str = buffer.decode('utf-8').strip()
                if end_str.startswith("{"):  # server whats to tell us something!
                    something_from_server = json.loads(end_str)
                    if "detail" in something_from_server:
                        raise RuntimeError(something_from_server["detail"])
                    print("SERVER SAYS:", end_str)
                for x in choice_collector.choices:
                    if x.content is not None and len(x.content) == 0:
                        x.content = None
                choices = [(x if x.content is not None or x.tool_calls is not None else None) for x in choice_collector.choices]
                # when streaming, subchats are streamed too
                has_home = set()
                for d in deterministic:
                    if d.tool_call_id is None:
                        continue
                    d.subchats = collections.defaultdict(list)
                    for k, msglist in subchats.items():
                        if k.startswith(d.tool_call_id + "__"):
                            subchat_id = k[len(d.tool_call_id + "__"):]
                            d.subchats[subchat_id] = msglist
                            has_home.add(k)
                assert set(has_home) == set(subchats.keys()), f"Whoops, not all subchats {subchats.keys()} are attached to a tool result."
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
        messages=messages_to_dicts(messages, verbose, tools=tools, temperature=temperature, model_name=model_name)[0],
        temperature=temperature,
        top_p=0.95,
        stop=stop,
        stream=False,
        tools=tools,
        max_tokens=max_tokens
    )
    assert isinstance(
        chat_completion, openai.types.chat.chat_completion.ChatCompletion)
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
    choices_not_none: List[Message] = [x for x in choices if x is not None]
    return join_messages_and_choices(messages, deterministic, choices_not_none, verbose)


async def diff_apply(
    base_url: str,
    chunks: List[Dict[str, Any]],
    apply: List[bool],
) -> List[List[Message]]:
    post_me = {
        "apply": apply,
        "chunks": chunks,
    }
    async with aiohttp.ClientSession() as session:
        async with session.post(base_url + "/diff-apply", json=post_me) as response:
            if response.status != 200:
                raise Exception(f"unexpected response status {response.status}, response: {await response.text()}")
            return await response.json(content_type=None)


async def mem_add(base_url: str, mem_type: str, goal: str, project: str, payload: str) -> Dict[str, Any]:
    url = f"{base_url}/mem-add"
    data = {
        "mem_type": mem_type,
        "goal": goal,
        "project": project,
        "payload": payload
    }
    async with aiohttp.ClientSession() as session:
        async with session.post(url, json=data) as response:
            return await response.json()


async def mem_block_until_vectorized(base_url: str) -> Tuple[Dict[str, Any], float]:
    url = f"{base_url}/mem-block-until-vectorized"
    t0 = time.time()
    async with aiohttp.ClientSession() as session:
        async with session.get(url) as response:
            return (await response.json(), time.time() - t0)


async def mem_update_used(base_url: str, memid: str, correct: float, relevant: float) -> Dict[str, Any]:
    url = f"{base_url}/mem-update-used"
    data = {
        "memid": memid,
        "correct": correct,
        "relevant": relevant
    }
    async with aiohttp.ClientSession() as session:
        async with session.post(url, json=data) as response:
            return await response.json()


async def mem_erase(base_url: str, memid: str) -> Dict[str, Any]:
    url = f"{base_url}/mem-erase"
    data = {
        "memid": memid
    }
    async with aiohttp.ClientSession() as session:
        async with session.post(url, json=data) as response:
            return await response.json()


async def mem_query(base_url: str, goal: str, project: str, top_n: Optional[int] = 5) -> Tuple[int, Dict[str, Any]]:
    url = f"{base_url}/mem-query"
    data = {
        "goal": goal,
        "project": project,
        "top_n": top_n
    }
    async with aiohttp.ClientSession() as session:
        async with session.post(url, json=data) as response:
            return response.status, await response.json()


async def ongoing_update(base_url: str, goal: str, progress: Dict[str, Any], actseq: Dict[str, Any], output: Dict[str, Any]):
    url = f"{base_url}/ongoing-update"
    data = {
        "goal": goal,
        "ongoing_progress": progress,
        "ongoing_action_new_sequence": actseq,
        "ongoing_output": output,
    }
    async with aiohttp.ClientSession() as session:
        async with session.post(url, json=data) as response:
            return await response.json()


def gen_function_call_id():
    return f"call_{uuid.uuid4()}".replace("-", "")


def pretend_function_call(tool_name, tool_arguments):
    tool_call_id = gen_function_call_id()
    tool_call = {
        "id": tool_call_id,
        "function": {
            "arguments": json.dumps(tool_arguments),
            "name": tool_name
        },
        "type": "function"
    }
    return tool_call


def print_block(
    name: str,
    n: int,
    width: int = 90,
    also_print_to_console: bool = True,
) -> str:
    block_text = f"{name.upper()} {n}"
    left_padding = " " * ((width - len(block_text)) // 2)
    right_padding = " " * (width - len(block_text) - len(left_padding))
    block_text = left_padding + block_text + right_padding

    tabulate.PRESERVE_WHITESPACE = True
    message = f"\n\n{tabulate.tabulate([[block_text]], tablefmt='double_grid')}\n\n"
    tabulate.PRESERVE_WHITESPACE = False

    if also_print_to_console:
        console = Console()
        console.print(message)

    return message


def print_messages(
    messages: List[Message],
    also_print_to_console: bool = True,
) -> List[str]:
    console: Optional[Console] = Console() if also_print_to_console else None

    def con(x):
        if console:
            console.print(x)

    def _is_tool_call(m: Message) -> bool:
        return m.tool_calls is not None and len(m.tool_calls) > 0

    def _wrap_color(s: str, color: str = "red") -> str:
        return f"[bold {color}]{s}[/bold {color}]"

    results = []
    role_to_header = {
        "system": "SYSTEM:",
        "assistant": "ASSISTANT:",
        "user": "USER:",
        "tool": "TOOL ANSWER id={uid}:",
        "context_file": "CONTEXT FILE:",
        "diff": "DIFF:",
    }
    for m in messages:
        message_str = []

        header = role_to_header.get(m.role, m.role.upper())
        if m.role == "tool":
            header = header.format(uid=m.tool_call_id[:20])
        message_str.append(header)
        con(_wrap_color(header))

        if m.role == "context_file" and m.content is not None:
            context_file = json.loads(m.content)
            for fdict in context_file:
                t = f"{fdict['file_name']}:{fdict['line1']}-{fdict['line2']}, len={len(fdict['file_content'])}\n"
                t += fdict['file_content']
                message_str.append(t)
                con(t)

        elif m.role == "diff" and m.content is not None:
            for chunk in json.loads(m.content):
                message = f"{chunk['file_name']}:{chunk['line1']}-{chunk['line2']}"
                message_str.append(message)
                con(message)
                if len(chunk["lines_add"]) > 0:
                    message = "\n".join([f"+{line}" for line in chunk['lines_add'].splitlines()])
                    message_str.append(message)
                    con(_wrap_color(message, "green"))
                if len(chunk["lines_remove"]) > 0:
                    message = "\n".join([f"-{line}" for line in chunk['lines_remove'].splitlines()])
                    message_str.append(message)
                    con(_wrap_color(message, "red"))

        elif m.role in ["tool", "user", "assistant", "system"]:
            if m.subchats:  # actually subchats can only appear in role="tool", but code is the same anyway
                for subchat_id, subchat_msgs in m.subchats.items():
                    subchats_strs = print_messages(subchat_msgs, also_print_to_console=also_print_to_console)
                    subchats_str = "\n".join(subchats_strs)
                    subchats_str = "\n".join([f" - {subchat_id} -   {line}" for line in subchats_str.splitlines()])
                    message_str.append(subchats_str)
            if m.content is not None:
                message_str.append(m.content)
                if m.content.startswith("[") or m.content.startswith("{"):
                    con(m.content)
                else:
                    con(Markdown(m.content))

        else:
            t = "unknown message role=\"%s\"" % m.role
            message_str.append(t)
            con(t)

        if m.tool_calls is not None:
            if not _is_tool_call(m):
                results.append("\n".join(message_str))
                continue
            t = "\n".join([
                f"{tool_call.function.name}({tool_call.function.arguments}) [id={tool_call.id[:20]}]"
                for tool_call in m.tool_calls
            ])
            message_str.append(t)
            con(t)

        message_str.append("")
        con("")

        results.append("\n".join(message_str))

    return results
