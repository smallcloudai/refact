import time
import json
import copy
import asyncio
import aiohttp
import termcolor
import os
import re
import litellm
import traceback

from fastapi import APIRouter, Request, HTTPException, Query, Header
from fastapi.responses import Response, StreamingResponse

from refact_utils.scripts import env
from refact_utils.finetune.utils import running_models_and_loras
from refact_webgui.webgui.selfhost_model_resolve import completion_resolve_model
from refact_webgui.webgui.selfhost_model_resolve import static_resolve_model
from refact_webgui.webgui.selfhost_queue import Ticket
from refact_webgui.webgui.selfhost_webutils import log
from refact_webgui.webgui.selfhost_queue import InferenceQueue
from refact_webgui.webgui.selfhost_model_assigner import ModelAssigner
from refact_webgui.webgui.selfhost_login import RefactSession

from pydantic import BaseModel, Required
from typing import List, Dict, Union, Optional, Tuple, Any

__all__ = ["BaseCompletionsRouter", "CompletionsRouter"]


def clamp(lower, upper, x):
    return max(lower, min(upper, x))


def red_time(base_ts):
    return termcolor.colored("%0.1fms" % (1000*(time.time() - base_ts)), "red")


def chat_limit_messages(messages: List[Dict[str, str]]):
    if len(messages) == 0:
        raise HTTPException(status_code=400, detail="No messages")
    while len(messages) > 10:
        del messages[0:2]  # user, assistant
    while sum([len(m["content"] + m["role"]) for m in messages]) > 8000:
        del messages[0:2]  # user, assistant
    return messages


class NlpSamplingParams(BaseModel):
    max_tokens: int = 500
    temperature: float = 0.2
    top_p: float = 1.0
    top_n: int = 0
    stop: Union[List[str], str] = []

    def clamp(self):
        self.temperature = clamp(0, 4, self.temperature)
        self.top_p = clamp(0.0, 1.0, self.top_p)
        self.top_n = clamp(0, 1000, self.top_n)
        self.max_tokens = clamp(0, 8192, self.max_tokens)
        return {
            "temperature": self.temperature,
            "top_p": self.top_p,
            "top_n": self.top_n,
            "max_tokens": self.max_tokens,
            "created": time.time(),
            "stop_tokens": self.stop,
        }


class NlpCompletion(NlpSamplingParams):
    model: str = Query(default=Required, regex="^[a-z/A-Z0-9_\.\-\:]+$")
    prompt: str
    n: int = 1
    echo: bool = False
    stream: bool = False
    mask_emails: bool = False


class ChatMessage(BaseModel):
    role: str
    content: str


class ChatContext(NlpSamplingParams):
    messages: List[ChatMessage]
    n: int = 1
    model: str = Query(default=Required, regex="^[a-z/A-Z0-9_\.\-]+$")
    function: str = Query(default="chat", regex="^[a-zA-Z0-9_\.\-]+$")


class EmbeddingsStyleOpenAI(BaseModel):
    input: Union[str, List[str]]
    model: str = Query(default=Required, regex="^[a-z/A-Z0-9_\.\-]+$")


def _mask_emails(text: str, mask: str = "john@example.com") -> str:
    masked_text = text
    for m in re.findall(r'[\w.+-]+@[\w-]+\.[\w.-]+', text):
        masked_text = masked_text.replace(m, mask)
    return masked_text


async def _completion_streamer(ticket: Ticket, post: NlpCompletion, timeout, seen, created_ts, caps_version: int):
    try:
        packets_cnt = 0
        while 1:
            try:
                msg = await asyncio.wait_for(ticket.streaming_queue.get(), timeout)
            except asyncio.TimeoutError:
                log("TIMEOUT %s" % ticket.id())
                msg = {"status": "error", "human_readable_message": "timeout"}
            not_seen_resp = copy.deepcopy(msg)
            not_seen_resp["caps_version"] = caps_version
            is_final_msg = msg.get("status", "") != "in_progress"
            if "choices" in not_seen_resp:
                for i in range(post.n):
                    newtext = not_seen_resp["choices"][i]["text"]
                    if newtext.startswith(seen[i]):
                        delta = newtext[len(seen[i]):]
                        if " " not in delta and not is_final_msg:
                            not_seen_resp["choices"][i]["text"] = ""
                            continue
                        if post.mask_emails:
                            if not is_final_msg:
                                delta = " ".join(delta.split(" ")[:-1])
                            not_seen_resp["choices"][i]["text"] = _mask_emails(delta)
                        else:
                            not_seen_resp["choices"][i]["text"] = delta
                        if post.stream:
                            seen[i] = newtext[:len(seen[i])] + delta
                    else:
                        log("ooops seen doesn't work, might be infserver's fault")
            if not post.stream:
                if not is_final_msg:
                    continue
                yield json.dumps(not_seen_resp)
                break
            yield "data: " + json.dumps(not_seen_resp) + "\n\n"
            packets_cnt += 1
            if is_final_msg:
                break
        if post.stream:
            yield "data: [DONE]" + "\n\n"
        log(red_time(created_ts) + " /finished %s, streamed %i packets" % (ticket.id(), packets_cnt))
        ticket.done()
        # fastapi_stats.stats_accum[kt] += msg.get("generated_tokens_n", 0)
        # fastapi_stats.stats_accum[kcomp] += 1
        # fastapi_stats.stats_lists_accum["stat_latency_" + post.model].append(time.time() - created_ts)
    finally:
        if ticket.id() is not None:
            log("   ***  CANCEL  ***  cancelling %s " % ticket.id() + red_time(created_ts))
            # fastapi_stats.stats_accum["stat_api_cancelled"] += 1
            # fastapi_stats.stats_accum["stat_m_" + post.model + "_cancelled"] += 1
        ticket.cancelled = True


async def chat_streamer(ticket: Ticket, timeout, created_ts):
    seen: Dict[int, str] = dict()
    try:
        while 1:
            try:
                msg: Dict = await asyncio.wait_for(ticket.streaming_queue.get(), timeout)
            except asyncio.TimeoutError:
                log("TIMEOUT %s" % ticket.id())
                msg = {"status": "error", "human_readable_message": "timeout"}
            if "choices" in msg:
                for ch in msg["choices"]:
                    idx = ch["index"]
                    seen_here = seen.get(idx, "")
                    content = ch.get("content", "")
                    ch["delta"] = content[len(seen_here):]
                    seen[idx] = content
                    if "content" in ch:
                        del ch["content"]
            tmp = json.dumps(msg)
            yield "data: " + tmp + "\n\n"
            log("  " + red_time(created_ts) + " stream %s <- %i bytes" % (ticket.id(), len(tmp)))
            if msg.get("status", "") != "in_progress":
                break
        await asyncio.sleep(0.5)   # a workaround for VS Code plugin bug, remove July 20, 2023 when plugin should be fixed
        yield "data: [DONE]" + "\n\n"
        log(red_time(created_ts) + " /finished call %s" % ticket.id())
        ticket.done()
    finally:
        if ticket.id() is not None:
            log("   ***  CANCEL  ***  cancelling %s" % ticket.id() + red_time(created_ts))
        ticket.cancelled = True
        ticket.done()


async def embeddings_streamer(ticket: Ticket, timeout, created_ts):
    try:
        while 1:
            try:
                msg: Dict = await asyncio.wait_for(ticket.streaming_queue.get(), timeout)
                msg['choices'] = msg['choices'][0]
                msg["files"] = [json.loads(v) for v in msg['choices']['files'].values()]
                del msg['choices']
            except asyncio.TimeoutError:
                log("TIMEOUT %s" % ticket.id())
                msg = {"status": "error", "human_readable_message": "timeout"}

            tmp = json.dumps(msg.get("files", []))
            yield tmp
            log("  " + red_time(created_ts) + " stream %s <- %i bytes" % (ticket.id(), len(tmp)))
            if msg.get("status", "") != "in_progress":
                break

        log(red_time(created_ts) + " /finished call %s" % ticket.id())
        ticket.done()
    finally:
        if ticket.id() is not None:
            log("   ***  CANCEL  ***  cancelling %s" % ticket.id() + red_time(created_ts))
        ticket.cancelled = True
        ticket.done()


async def error_string_streamer(ticket_id, static_message, account, created_ts):
    yield "data: " + json.dumps(
        {"object": "smc.chat.chunk", "role": "assistant", "delta": static_message, "finish_reason": "END"}) + "\n\n"
    yield "data: [ERROR]" + "\n\n"
    log("  " + red_time(created_ts) + "%s chat static message to %s: %s" % (ticket_id, account, static_message))


class BaseCompletionsRouter(APIRouter):

    def __init__(self,
                 inference_queue: InferenceQueue,
                 id2ticket: Dict[str, Ticket],
                 model_assigner: ModelAssigner,
                 timeout: int = 30,
                 *args, **kwargs):
        super().__init__(*args, **kwargs)

        # API for direct FIM and Chat usage
        self.add_api_route("/v1/login", self._login, methods=["GET"])
        self.add_api_route("/v1/secret-key-activate", self._secret_key_activate, methods=["GET"])
        self.add_api_route("/v1/chat", self._chat, methods=["POST"])

        # API for LSP server
        self.add_api_route("/coding_assistant_caps.json", self._coding_assistant_caps, methods=["GET"])
        self.add_api_route("/v1/completions", self._completions, methods=["POST"])
        self.add_api_route("/v1/embeddings", self._embeddings_style_openai, methods=["POST"])

        self.add_api_route("/v1/models", self._models, methods=["GET"])
        self.add_api_route("/v1/chat/completions", self._chat_completions, methods=["POST"])

        self._inference_queue = inference_queue
        self._id2ticket = id2ticket
        self._model_assigner = model_assigner
        self._timeout = timeout

    async def _account_from_bearer(self, authorization: str) -> str:
        raise NotImplementedError()

    @staticmethod
    def _integrations_env_setup():
        inference = {}
        if os.path.exists(env.CONFIG_INFERENCE):
            inference = json.load(open(env.CONFIG_INFERENCE, 'r'))
        integrations = {}
        if os.path.exists(env.CONFIG_INTEGRATIONS):
            integrations = json.load(open(env.CONFIG_INTEGRATIONS, 'r'))

        def _integrations_env_setup(env_var_name: str, api_key_name: str, api_enable_name: str):
            os.environ[env_var_name] = integrations.get(api_key_name, "") if inference.get(api_enable_name, False) else ""

        _integrations_env_setup("OPENAI_API_KEY", "openai_api_key", "openai_api_enable")
        _integrations_env_setup("ANTHROPIC_API_KEY", "anthropic_api_key", "anthropic_api_enable")

    async def _coding_assistant_caps_base_data(self) -> Dict[str, Any]:
        running = running_models_and_loras(self._model_assigner)
        models_available = self._inference_queue.models_available(force_read=True)
        code_completion_default_model, _ = completion_resolve_model(self._inference_queue)
        code_chat_default_model = ""
        embeddings_default_model = ""
        for model_name in models_available:
            if "chat" in self._model_assigner.models_db.get(model_name, {}).get("filter_caps", []) or model_name in litellm.model_list:
                if not code_chat_default_model:
                    code_chat_default_model = model_name
            if "embeddings" in self._model_assigner.models_db.get(model_name, {}).get("filter_caps", []):
                if not embeddings_default_model:
                    embeddings_default_model = model_name
        config_mtime = self._model_assigner.config_inference_mtime()
        data = {
            "cloud_name": "Refact Self-Hosted",
            "endpoint_template": "/v1/completions",
            "endpoint_chat_passthrough": "/v1/chat/completions",
            "endpoint_style": "openai",
            "telemetry_basic_dest": "/stats/telemetry-basic",
            "telemetry_corrected_snippets_dest": "/stats/telemetry-snippets",
            "telemetry_basic_retrieve_my_own": "/stats/rh-stats",
            "running_models": [r for r in [*running['completion'], *running['chat']]],
            "code_completion_default_model": code_completion_default_model,
            "code_chat_default_model": code_chat_default_model,

            "default_embeddings_model": embeddings_default_model,
            "endpoint_embeddings_template": "v1/embeddings",
            "endpoint_embeddings_style": "openai",
            "size_embeddings": 768,

            "tokenizer_path_template": "https://huggingface.co/$MODEL/resolve/main/tokenizer.json",
            "tokenizer_rewrite_path": {
                model: self._model_assigner.models_db[model]["model_path"]
                for model in models_available
                if model in self._model_assigner.models_db
            },
            "caps_version": config_mtime,
        }

        return data

    async def _coding_assistant_caps(self, request: Request, authorization: str = Header(None)):
        client_version = request.headers.get("client_version", "0")

        data = await self._coding_assistant_caps_base_data()
        if client_version >= "0.7.1":
            running = running_models_and_loras(self._model_assigner)

            if cc_default := data.get("code_completion_default_model"):
                if cc_variants := [r for r in running['completion'] if r.split(":")[0] == cc_default and r != cc_default]:
                    data["code_completion_default_model"] = cc_variants[0]

            if cc_chat_default := data.get("code_chat_default_model"):
                if cc_variants := [r for r in running['chat'] if r.split(':')[0] == cc_chat_default and r != cc_chat_default]:
                    data["code_chat_default_model"] = cc_variants[0]
        else:
            log(f"refact-lsp version {client_version} is deprecated, finetune is unavailable. Update your plugin")

        return Response(content=json.dumps(data, indent=4), media_type="application/json")

    async def _login(self, authorization: str = Header(None)):
        await self._account_from_bearer(authorization)

        longthink_functions = dict()
        longthink_filters = set()
        models_mini_db_extended = {
            "longthink/stable": {
                "filter_caps": ["gpt3.5", "gpt4"],
            },
            **self._model_assigner.models_db,
        }
        filter_caps = set([
            capability
            for model in self._inference_queue.models_available(force_read=True)
            for capability in models_mini_db_extended.get(model, {}).get("filter_caps", [])
        ])
        for rec in self._model_assigner.models_caps_db:
            rec_modelcaps = rec.model if isinstance(rec.model, list) else [rec.model]
            rec_third_parties = rec.third_party if isinstance(rec.third_party, list) else [rec.third_party]
            for rec_modelcap, rec_third_party in zip(rec_modelcaps, rec_third_parties):
                if rec_modelcap not in filter_caps:
                    continue
                rec_modelcap = rec_modelcap.replace("/", "-")
                if rec_third_party:
                    rec_model = rec_modelcap
                else:
                    rec_model, err_msg = static_resolve_model(rec_modelcap, self._inference_queue)
                    assert err_msg == "", err_msg
                rec_function_name = f"{rec.function_name}-{rec_modelcap}"
                longthink_functions[rec_function_name] = {
                    **rec.to_dict(),
                    "function_name": rec_function_name,
                    "is_liked": False,
                    "likes": 0,
                    "third_party": rec_third_party,
                    "model": rec_model,
                }
                if "/" not in rec_model:
                    longthink_filters.add(rec_model)
        return {
            "account": "self-hosted",
            "retcode": "OK",
            "longthink-functions-today": 1,
            "longthink-functions-today-v2": longthink_functions,
            "longthink-filters": list(longthink_filters),
            "chat-v1-style": 1,
        }

    async def _secret_key_activate(self, authorization: str = Header(None)):
        await self._account_from_bearer(authorization)
        return {
            "retcode": "OK",
            "human_readable_message": "API key verified",
        }

    async def _resolve_model_lora(self, model_name: str) -> Tuple[str, Optional[Dict[str, str]]]:
        running = running_models_and_loras(self._model_assigner)
        if model_name not in {r for r in [*running['completion'], *running['chat']]}:
            return model_name, None

        model_name, run_id, checkpoint_id = (*model_name.split(":"), None, None)[:3]
        if run_id is None or checkpoint_id is None:
            return model_name, None

        return model_name, {
            "run_id": run_id,
            "checkpoint_id": checkpoint_id,
        }

    async def _completions(self, post: NlpCompletion, authorization: str = Header(None)):
        account = await self._account_from_bearer(authorization)

        ticket = Ticket("comp-")
        req = post.clamp()
        caps_version = self._model_assigner.config_inference_mtime()       # use mtime as a version, if that changes the client will know to refresh caps

        model_name, lora_config = await self._resolve_model_lora(post.model)
        model_name, err_msg = static_resolve_model(model_name, self._inference_queue)

        if err_msg:
            log("%s model resolve \"%s\" -> error \"%s\" from %s" % (ticket.id(), post.model, err_msg, account))
            return Response(status_code=400, content=json.dumps({"detail": err_msg, "caps_version": caps_version}, indent=4), media_type="application/json")

        if lora_config:
            log(f'{ticket.id()} model resolve "{post.model}" -> "{model_name}" lora {lora_config} from {account}')
        else:
            log(f'{ticket.id()} model resolve "{post.model}" -> "{model_name}" from {account}')

        req.update({
            "object": "text_completion_req",
            "account": account,
            "prompt": post.prompt,
            "model": model_name,
            "stream": post.stream,
            "echo": post.echo,
            "lora_config": lora_config,
        })
        ticket.call.update(req)
        q = self._inference_queue.model_name_to_queue(ticket, model_name)
        self._id2ticket[ticket.id()] = ticket
        await q.put(ticket)
        seen = [""] * post.n
        return StreamingResponse(
            _completion_streamer(ticket, post, self._timeout, seen, req["created"], caps_version=caps_version),
            media_type=("text/event-stream" if post.stream else "application/json"),
        )

    # deprecated, no loras
    async def _chat(self, post: ChatContext, request: Request, authorization: str = Header(None)):
        account = await self._account_from_bearer(authorization)

        ticket = Ticket("comp-")

        model_name, err_msg = static_resolve_model(post.model, self._inference_queue)
        if err_msg:
            log("%s model resolve \"%s\" -> error \"%s\" from %s" % (ticket.id(), post.model, err_msg, account))
            raise HTTPException(status_code=400, detail=err_msg)
        log("%s chat model resolve \"%s\" -> \"%s\" from %s" % (ticket.id(), post.model, model_name, account))

        req = post.clamp()
        post_raw = await request.json()
        messages = chat_limit_messages(post_raw["messages"])
        if len(messages) == 0:
            return StreamingResponse(
                error_string_streamer(
                    ticket.id(), "Your messsage is too large, the limit is 4k characters", account, req["created"]))
        req.update({
            "id": ticket.id(),
            "object": "chat_completion_req",
            "account": account,
            "model": model_name,
            "function": post.function,
            "messages": messages,
            "stream": True,
        })

        ticket.call.update(req)
        q = self._inference_queue.model_name_to_queue(ticket, model_name)
        self._id2ticket[ticket.id()] = ticket
        await q.put(ticket)
        return StreamingResponse(chat_streamer(ticket, self._timeout, req["created"]))

    async def _generate_embeddings(self, account: str, inputs: Union[str, List[str]], model_name: str):
        if model_name not in self._inference_queue.models_available():
            log(f"model {model_name} is not running")
            raise HTTPException(status_code=400, detail=f"model {model_name} is not running")

        tickets, reqs = [], []
        inputs = inputs if isinstance(inputs, list) else [inputs]
        for inp in inputs:
            ticket = Ticket("embed-")
            req = {
                "inputs": inp,
            }
            req.update({
                "id": ticket.id(),
                "account": account,
                "object": "embeddings_req",
                "model": model_name,
                "stream": True,
                "created": time.time()
            })
            ticket.call.update(req)
            q = self._inference_queue.model_name_to_queue(ticket, model_name, no_checks=True)
            self._id2ticket[ticket.id()] = ticket
            await q.put(ticket)
            tickets.append(ticket)
            reqs.append(req)

        for idx, (ticket, req) in enumerate(zip(tickets, reqs)):
            async for resp in embeddings_streamer(ticket, 60, req["created"]):
                resp = json.loads(resp)
                embedding = []
                try:
                    embedding = resp[0]
                except IndexError:
                    pass
                yield {"embedding": embedding, "index": idx}

    async def _embeddings_style_openai(self, post: EmbeddingsStyleOpenAI, authorization: str = Header(None)):
        account = await self._account_from_bearer(authorization)
        data = [
            {
                "embedding": res["embedding"],
                "index": res["index"],
                "object": "embedding",
            }
            async for res in self._generate_embeddings(account, post.input, post.model)
        ]
        data.sort(key=lambda x: x["index"])

        return {
            "data": data,
            "model": post.model,
            "object": "list",
            "usage": {"prompt_tokens": -1, "total_tokens": -1}
        }

    async def _models(self, authorization: str = Header(None)):
        await self._account_from_bearer(authorization)
        try:
            async with aiohttp.ClientSession() as session:
                async with session.get("http://127.0.0.1:8001/v1/caps") as resp:
                    lsp_server_caps = await resp.json()
        except aiohttp.ClientConnectorError as e:
            err_msg = f"LSP server is not ready yet: {e}"
            log(err_msg)
            raise HTTPException(status_code=401, detail=err_msg)
        completion_models = set()
        for model, caps in lsp_server_caps["code_completion_models"].items():
            completion_models.update({model, *caps["similar_models"]})
        chat_models = set()
        for model, caps in lsp_server_caps["code_chat_models"].items():
            chat_models.update({model, *caps["similar_models"]})
        data = [
            {
                "id": model, "root": model, "object": "model",
                "created": 0, "owned_by": "", "permission": [], "parent": None,
                "completion": model in completion_models, "chat": model in chat_models,
            }
            for model in lsp_server_caps["running_models"]
        ]
        return {
            "object": "list",
            "data": data,
        }

    async def _chat_completions(self, post: ChatContext, authorization: str = Header(None)):
        account = await self._account_from_bearer(authorization)

        prefix, postfix = "data: ", "\n\n"

        if post.model in litellm.model_list:
            async def litellm_streamer(post: ChatContext):
                try:
                    self._integrations_env_setup()
                    response = await litellm.acompletion(
                        model=post.model, messages=[m.dict() for m in post.messages], stream=True,
                        temperature=post.temperature, top_p=post.top_p, max_tokens=post.max_tokens, stop=post.stop)
                    finish_reason = None
                    async for model_response in response:
                        try:
                            data = model_response.dict()
                            finish_reason = data["choices"][0]["finish_reason"]
                        except json.JSONDecodeError:
                            data = {"choices": [{"finish_reason": finish_reason}]}
                        yield prefix + json.dumps(data) + postfix
                    # NOTE: DONE needed by refact-lsp server
                    yield prefix + "[DONE]" + postfix
                except BaseException as e:
                    err_msg = f"litellm error: {e}"
                    log(err_msg)
                    yield prefix + json.dumps({"error": err_msg}) + postfix

            response_streamer = litellm_streamer(post)

        else:
            async def chat_completion_streamer(post: ChatContext):
                post_url = "http://127.0.0.1:8001/v1/chat"
                post_data = {
                    "messages": [m.dict() for m in post.messages],
                    "stream": True,
                    "model": post.model,
                    "parameters": {
                        "temperature": post.temperature,
                        "max_new_tokens": post.max_tokens,
                    }
                }
                async with aiohttp.ClientSession() as session:
                    try:
                        async with session.post(post_url, json=post_data) as response:
                            finish_reason = None
                            async for data, _ in response.content.iter_chunks():
                                try:
                                    data = data.decode("utf-8")
                                    data = json.loads(data[len(prefix):-len(postfix)])
                                    finish_reason = data["choices"][0]["finish_reason"]
                                    data["choices"][0]["finish_reason"] = None
                                except json.JSONDecodeError:
                                    data = {"choices": [{"finish_reason": finish_reason}]}
                                yield prefix + json.dumps(data) + postfix
                    except aiohttp.ClientConnectorError as e:
                        err_msg = f"LSP server is not ready yet: {e}"
                        log(err_msg)
                        yield prefix + json.dumps({"error": err_msg}) + postfix

            response_streamer = chat_completion_streamer(post)

        return StreamingResponse(response_streamer, media_type="text/event-stream")


class CompletionsRouter(BaseCompletionsRouter):
    def __init__(self, session: RefactSession, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self._session = session

    async def _account_from_bearer(self, authorization: str) -> str:
        try:
            return self._session.header_authenticate(authorization)
        except Exception as e:
            traceback_str = traceback.format_exc()
            log(traceback_str)
            raise HTTPException(status_code=401, detail=str(e))
