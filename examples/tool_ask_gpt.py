from os import getenv
from typing import List, Tuple, Dict, Any

import openai
import termcolor
from openai import OpenAIError

aclient = openai.AsyncOpenAI(
    base_url="http://127.0.0.1:8001/v1",
    # base_url="https://openrouter.ai/api/v1",
    # api_key=getenv("OPENROUTER_API_KEY"),
    api_key=getenv("OPENAI_API_KEY"),
)
MAX_RETRIES = 1


async def simple_ask_gpt(
        logindent: str,
        messages: List[Tuple[str, str]],
        n_answers: int,
        stop: List[str],
        verbose: bool,
        tools: List[Dict[str, Any]] = [],
        tool_choice: str = "auto",
        model_name: str = "gpt-3.5-turbo-0125",
        temperature: float = 0.85
) -> List[Tuple[str, str]]:
    if verbose:
        print(termcolor.colored(logindent, "blue"), termcolor.colored("------ simple_ask_gpt %s T=%0.2f ------" % (model_name, temperature), "red"))
        for msg in messages:
            if msg[0] == "system":
                continue
            print(termcolor.colored(logindent, "blue"), termcolor.colored(msg[0], "yellow"), msg[1].replace("\n", "\\n"))

    retries = MAX_RETRIES
    while retries:
        try:
            chat_completion = await aclient.chat.completions.create(
                model=model_name,
                n=n_answers,
                messages=[{"role": x[0], "content": x[1]} for x in messages],  # type: ignore
                temperature=temperature,
                top_p=0.95,
                stop=stop,
                stream=True,
                tools=tools,
                tool_choice=tool_choice,
                # extra_headers={
                #     "HTTP-Referer": "https://github.com/alonsosilvaallende/chatplotlib-openrouter"
                # }
            )
            # assert isinstance(chat_completion, openai.types.chat.chat_completion.ChatCompletion)
            async for xxx in chat_completion:
                print(xxx)
            result = [("", "")] * len(chat_completion.choices)
            # print(chat_completion)
            for i, ch in enumerate(chat_completion.choices):
                print("QQQQ", ch)
                if isinstance(ch.message.content, str):  # regular answer
                    content: str = ch.message.content
                    finish_reason = ch.finish_reason
                    result[i] = (content, finish_reason)
                elif isinstance(ch.message.tool_calls, list):  # tool answer
                    for tcall in ch.message.tool_calls:
                        print("AAAA", tcall)
            choice0 = chat_completion.choices[0]
            assert isinstance(choice0.message.content, str)
            if verbose:
                for i, r in enumerate(result):
                    print(termcolor.colored(logindent, "blue"),
                          termcolor.colored("result[%d] => %s" % (i, r[0]), "yellow"),
                          termcolor.colored(r[1], "red"))
            return result
        except OpenAIError as e:
            print(e)
            retries -= 1
            continue
    else:
        raise OpenAIError("Too many retries")
