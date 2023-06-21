import time
import json
import copy
import asyncio
import termcolor

from fastapi import APIRouter, Request, HTTPException, Query
from fastapi.responses import StreamingResponse

from code_contrast.model_caps import modelcap_records
from refact_self_hosting.known_models import models_mini_db
from refact_self_hosting.webgui import selfhost_model_resolve
from refact_self_hosting.webgui.selfhost_req_queue import Ticket
from refact_self_hosting.webgui.selfhost_webutils import log

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
    while sum([len(m["content"] + m["role"]) for m in messages]) > 4000:
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
    model: str = Query(default="", regex="^[a-z/A-Z0-9_\.]+$")
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
    model: str = Query(default="", regex="^[a-z/A-Z0-9_\.]+$")
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
    function: str = Query(default="chat", regex="^([a-z0-9\.\-]+)$")


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
            log("  " + red_time(created_ts) + "stream %s <- %i bytes" % (ticket.id(), len(tmp)))
            if msg.get("status", "") != "in_progress":
                break
        yield "data: [DONE]" + "\n\n"
        log(red_time(created_ts) + "/finished call %s" % ticket.id())
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
                 user2gpu_queue: Dict[str, asyncio.Queue],
                 id2ticket: Dict[str, Ticket],
                 timeout: int = 30,
                 *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.add_api_route("/login", self._longthink_functions, methods=["GET"])
        self.add_api_route("/secret-key-activate", self._secret_key_activate, methods=["GET"])
        self.add_api_route("/completions", self._completions, methods=["POST"])
        self.add_api_route("/contrast", self._contrast, methods=["POST"])
        self.add_api_route("/chat", self._chat, methods=["POST"])
        self._user2gpu_queue = user2gpu_queue
        self._id2ticket = id2ticket
        self._timeout = timeout

    async def _longthink_functions(self):
        longthink_functions = dict()
        models_mini_db_extended = {
            "longthink/stable": {
                "filter_caps": ["gpt3.5", "gpt4"],
            },
            **models_mini_db,
        }
        filter_caps = set([
            capability
            for model in self._user2gpu_queue
            for capability in models_mini_db_extended[model]["filter_caps"]])
        for rec in modelcap_records.db:
            rec_capabilities = rec.model if isinstance(rec.model, list) else [rec.model]
            rec_third_parties = rec.third_party if isinstance(rec.third_party, list) else [rec.third_party]
            for rec_capability, rec_third_party in zip(rec_capabilities, rec_third_parties):
                if rec_capability not in filter_caps:
                    continue
                rec_capability = rec_capability.replace("/", "-")
                if rec_third_party:
                    rec_model = rec_capability
                    rec_function_name = f"{rec.function_name}-{rec_capability}"
                else:
                    # NOTE: we should to resolve only local models and it must be fixed ASAP
                    rec_model, err_msg = selfhost_model_resolve.resolve_model(rec_capability, "", "")
                    if err_msg:
                        log(f'login capability resolve "{rec_model}" -> error "{err_msg}"')
                        continue
                    rec_function_name = rec.function_name
                longthink_functions[rec_function_name] = {
                    **rec.to_dict(),
                    "is_liked": False,
                    "likes": 0,
                    "third_party": rec_third_party,
                    "model": rec_model,
                }
        return {
            "account": "self-hosted",
            "retcode": "OK",
            "longthink-functions-today": 1,
            "longthink-functions-today-v2": longthink_functions,
            "longthink-filters": [],
            "chat-v1-style": 1,
        }

    async def _secret_key_activate(self):
        return {
            "retcode": "OK",
            "human_readable_message": "API key verified",
        }

    async def _completions(self, post: NlpCompletion):
        account = "XXX"
        ticket = Ticket("comp-")
        req = post.clamp()
        model_name, err_msg = selfhost_model_resolve.resolve_model(post.model, "", "")
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
        if model_name not in self._user2gpu_queue:
            log("%s model \"%s\" is not working at this moment" % (ticket.id(), model_name))
            raise HTTPException(status_code=400, detail="no model '%s' found." % model_name)
        self._id2ticket[ticket.id()] = ticket
        q = self._user2gpu_queue[model_name]
        await q.put(ticket)
        seen = [""] * post.n
        return StreamingResponse(completion_streamer(ticket, post, self._timeout, seen, req["created"]))

    async def _contrast(self, post: DiffCompletion, request: Request):
        account = "XXX"
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
        model_name, err_msg = selfhost_model_resolve.resolve_model(post.model, post.cursor_file, post.function)
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
        if model_name not in self._user2gpu_queue:
            log("%s model \"%s\" is not working at this moment" % (ticket.id(), model_name))
            raise HTTPException(status_code=400, detail="no model '%s' found." % model_name)
        # kt, kcomp = await _model_hit(red, ticket, req, model_name, account)
        self._id2ticket[ticket.id()] = ticket
        q = self._user2gpu_queue[model_name]
        await q.put(ticket)
        return StreamingResponse(diff_streamer(ticket, post, self._timeout, req["created"]))

    async def _chat(self, post: ChatContext, request: Request):
        account = "XXX"
        ticket = Ticket("comp-")

        model_name, err_msg = selfhost_model_resolve.resolve_model(post.model, "", post.function)
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
        self._id2ticket[ticket.id()] = ticket
        q = self._user2gpu_queue[model_name]
        await q.put(ticket)
        return StreamingResponse(chat_streamer(ticket, self._timeout, req["created"]))
