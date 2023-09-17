import time
import json
import copy
import asyncio
import termcolor

from fastapi import APIRouter, Request, HTTPException, Query
from fastapi.responses import StreamingResponse

from self_hosting_machinery.webgui.selfhost_model_resolve import completion_resolve_model
from self_hosting_machinery.webgui.selfhost_model_resolve import static_resolve_model
from self_hosting_machinery.webgui.selfhost_req_queue import Ticket
from self_hosting_machinery.webgui.selfhost_webutils import log
from self_hosting_machinery.webgui.selfhost_queue import InferenceQueue
from self_hosting_machinery.webgui.selfhost_model_assigner import ModelAssigner

from pydantic import BaseModel, Required
from typing import List, Dict, Union


__all__ = ["CompletionsRouter"]


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
    model: str = Query(default=Required, regex="^[a-z/A-Z0-9_\.\-]+$")
    prompt: str
    n: int = 1
    echo: bool = False
    stream: bool = False


class POI(BaseModel):
    filename: str
    cursor0: int
    cursor1: int
    priority: float


class DiffCompletion(NlpSamplingParams):
    model: str = Query(default="", regex="^[a-z/A-Z0-9_\.\-]+$")
    intent: str
    sources: Dict[str, str]
    cursor_file: str
    cursor0: int
    cursor1: int
    function: str = Query(
        default=Required, regex="^([a-z0-9\.\-]+)$"
    )
    max_edits: int = 4
    stream: bool = False
    poi: List[POI] = []


class ChatMessage(BaseModel):
    role: str
    content: str


class ChatContext(NlpSamplingParams):
    messages: List[ChatMessage]
    n: int = 1
    model: str = Query(default=Required, regex="^[a-z/A-Z0-9_\.\-]+$")
    function: str = Query(default="chat", regex="^[a-zA-Z0-9_\.\-]+$")


async def completion_streamer(ticket: Ticket, post: NlpCompletion, timeout, seen, created_ts):
    try:
        packets_cnt = 0
        while 1:
            try:
                msg = await asyncio.wait_for(ticket.streaming_queue.get(), timeout)
            except asyncio.TimeoutError:
                log("TIMEOUT %s" % ticket.id())
                msg = {"status": "error", "human_readable_message": "timeout"}
            not_seen_resp = copy.deepcopy(msg)
            if "choices" in not_seen_resp:
                for i in range(post.n):
                    newtext = not_seen_resp["choices"][i]["text"]
                    if newtext.startswith(seen[i]):
                        l = len(seen[i])
                        tmp = not_seen_resp["choices"][i]["text"]
                        not_seen_resp["choices"][i]["text"] = tmp[l:]
                        if post.stream:
                            seen[i] = tmp
                    else:
                        log("ooops seen doesn't work, might be infserver's fault")
            if not post.stream:
                if msg.get("status", "") == "in_progress":
                    continue
                yield json.dumps(not_seen_resp)
                break
            yield "data: " + json.dumps(not_seen_resp) + "\n\n"
            packets_cnt += 1
            if msg.get("status", "") != "in_progress":
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


async def diff_streamer(ticket: Ticket, post: DiffCompletion, timeout, created_ts):
    try:
        while 1:
            try:
                msg = await asyncio.wait_for(ticket.streaming_queue.get(), timeout)
            except asyncio.TimeoutError:
                log("TIMEOUT %s" % ticket.id())
                msg = {"status": "error", "human_readable_message": "timeout"}
            if not post.stream:
                if msg.get("status", "") == "in_progress":
                    continue
                yield json.dumps(msg)
                break
            tmp = json.dumps(msg)
            yield "data: " + tmp + "\n\n"
            log("  " + red_time(created_ts) + " stream %s <- %i bytes" % (ticket.id(), len(tmp)))
            if msg.get("status", "") != "in_progress":
                break
        if post.stream:
            yield "data: [DONE]" + "\n\n"
        log(red_time(created_ts) + " /finished call %s" % ticket.id())
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
        ticket.done()


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


async def error_string_streamer(ticket_id, static_message, account, created_ts):
    yield "data: " + json.dumps(
        {"object": "smc.chat.chunk", "role": "assistant", "delta": static_message, "finish_reason": "END"}) + "\n\n"
    yield "data: [ERROR]" + "\n\n"
    log("  " + red_time(created_ts) + "%s chat static message to %s: %s" % (ticket_id, account, static_message))


class CompletionsRouter(APIRouter):

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
        self.add_api_route("/v1/contrast", self._contrast, methods=["POST"])
        self.add_api_route("/v1/chat", self._chat, methods=["POST"])

        # API for LSP server
        self.add_api_route("/coding_assistant_caps.json", self._coding_assistant_caps, methods=["GET"])
        self.add_api_route("/v1/completions", self._completions, methods=["POST"])

        self._inference_queue = inference_queue
        self._id2ticket = id2ticket
        self._model_assigner = model_assigner
        self._timeout = timeout

    async def _coding_assistant_caps(self):
        code_completion_default_model, _ = completion_resolve_model(self._inference_queue)
        return {
            "cloud_name": "Refact Self-Hosted",
            "endpoint_template": "v1/completions",
            "endpoint_style": "openai",
            "tokenizer_path_template": "https://huggingface.co/$MODEL/resolve/main/tokenizer.json",
            "tokenizer_rewrite_path": {
                model: self._model_assigner.models_db[model]["model_path"]
                for model in self._inference_queue.models_available()
                if model in self._model_assigner.models_db
            },
            "telemetry_basic_dest": "http://localhost:8008/v1/telemetry-basic",
            "telemetry_corrected_snippets_dest": "http://localhost:8008/v1/telemetry-snippets",
            "code_completion_default_model": "smallcloudai/Refact-1_6B-fim",
            "code_chat_default_model": "llama2/7b",
            "running_models": ["smallcloudai/Refact-1_6B-fim", "llama2/7b"],
            "endpoint_chat_passthrough": ""
        }

    async def _login(self):
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
            for model in self._inference_queue.models_available()
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

    async def _secret_key_activate(self):
        return {
            "retcode": "OK",
            "human_readable_message": "API key verified",
        }

    async def _completions(self, post: NlpCompletion, account: str = "XXX"):
        ticket = Ticket("comp-")
        req = post.clamp()
        model_name, err_msg = static_resolve_model(post.model, self._inference_queue)
        if err_msg:
            log("%s model resolve \"%s\" -> error \"%s\" from %s" % (ticket.id(), post.model, err_msg, account))
            raise HTTPException(status_code=400, detail=err_msg)
        log("%s model resolve \"%s\" -> \"%s\" from %s" % (ticket.id(), post.model, model_name, account))
        req.update({
            "object": "text_completion_req",
            "account": account,
            "prompt": post.prompt,
            "model": model_name,
            "stream": post.stream,
            "echo": post.echo,
        })
        ticket.call.update(req)
        q = self._inference_queue.model_name_to_queue(ticket, model_name)
        self._id2ticket[ticket.id()] = ticket
        await q.put(ticket)
        seen = [""] * post.n
        return StreamingResponse(
            completion_streamer(ticket, post, self._timeout, seen, req["created"]),
            media_type=("text/event-stream" if post.stream else "application/json"),
        )

    async def _contrast(self, post: DiffCompletion, request: Request, account: str = "XXX"):
        if post.function != "diff-anywhere":
            if post.cursor_file not in post.sources:
                raise HTTPException(status_code=400, detail="cursor_file='%s' is not in sources=%s" % (post.cursor_file, list(post.sources.keys())))
            if post.cursor0 < 0 or post.cursor1 < 0:
                raise HTTPException(status_code=400, detail="cursor0=%d or cursor1=%d is negative" % (post.cursor0, post.cursor1))
            filetext = post.sources[post.cursor_file]
            if post.cursor0 > len(filetext) or post.cursor1 > len(filetext):
                raise HTTPException(status_code=400, detail="cursor0=%d or cursor1=%d is beyond file length=%d" % (post.cursor0, post.cursor1, len(filetext)))
        for fn, text in post.sources.items():
            if len(text) > 180*1024:
                raise HTTPException(status_code=400, detail="file '%s' is too long (%d bytes)" % (fn, len(text)))
        ticket = Ticket("comp-")
        if post.function == "infill":
            model_name, err_msg = completion_resolve_model(self._inference_queue)
        else:
            model_name, err_msg = static_resolve_model(post.model, self._inference_queue)
        if err_msg:
            log("%s model resolve \"%s\" func \"%s\" -> error \"%s\" from %s" % (ticket.id(), post.model, post.function, err_msg, account))
            raise HTTPException(status_code=400, detail=err_msg)
        log("%s model resolve \"%s\" func \"%s\" -> \"%s\" from %s" % (ticket.id(), post.model, post.function, model_name, account))
        if post.function == "highlight":
            post.max_tokens = 0
        req = post.clamp()
        req.update({
            "object": "diff_completion_req",
            "account": account,
            "model": model_name,
            "intent": post.intent,
            "sources": post.sources,
            "cursor_file": post.cursor_file,
            "cursor0": post.cursor0,
            "cursor1": post.cursor1,
            "function": post.function,
            "max_edits": post.max_edits,
            "stream": post.stream,
        })
        post_raw = await request.json()
        if "poi" in post_raw:
            req["poi"] = post_raw["poi"]
        ticket.call.update(req)
        q = self._inference_queue.model_name_to_queue(ticket, model_name)
        # kt, kcomp = await _model_hit(red, ticket, req, model_name, account)
        self._id2ticket[ticket.id()] = ticket
        await q.put(ticket)
        return StreamingResponse(diff_streamer(ticket, post, self._timeout, req["created"]))

    async def _chat(self, post: ChatContext, request: Request, account: str = "XXX"):
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
