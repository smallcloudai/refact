import time, threading, json, copy, asyncio, termcolor, collections
from fastapi import APIRouter, Request, Header, HTTPException, Query
from fastapi.responses import StreamingResponse
from refact_self_hosting import known_models
from refact_self_hosting.webgui import selfhost_req_queue
from refact_self_hosting.webgui.selfhost_webutils import clamp, log
from pydantic import BaseModel, Required
from typing import List, Dict, Tuple, Optional, Callable, Union, Any


TIMEOUT = 30


router = APIRouter()


@router.get("/secret-key-activate")
async def secret_key_activate(
    request: Request,
    authorization: str = Header(None),
):
    # red = fu.get_inf_red()
    # ip = request.client.host
    # _ac_dict = await fastapi_auth.lookup_bearer(authorization, red, force_www_req=True, ip=ip)
    return {
        "retcode": "OK",
        "human_readable_message": "API key verified",
    }


def red_time(base_ts):
    return termcolor.colored("%0.1fms" % (1000*(time.time() - base_ts)), "red")


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


@router.post("/completions")
async def completions(
    post: NlpCompletion,
    request: Request,
    authorization: str = Header(None),
):
    # ip = request.client.host
    # ac_dict = await fastapi_auth.lookup_bearer(authorization, red, ip=ip)
    # account = ac_dict["account"]
    account = "XXX"
    ticket = selfhost_req_queue.Ticket("comp-")
    req = post.clamp()
    model_name, err_msg = known_models.resolve_model(post.model, "", "")
    if err_msg:
        log("%s model resolve \"%s\" -> error \"%s\" from %s" % (ticket.ticket, post.model, err_msg, account))
        raise HTTPException(status_code=400, detail=err_msg)
    log("%s model resolve \"%s\" -> \"%s\" from %s" % (ticket.ticket, post.model, model_name, account))
    req.update({
        "id": ticket.ticket,
        "object": "text_completion_req",
        "account": account,
        "prompt": post.prompt,
        "model": model_name,
        "stream": post.stream,
        "echo": post.echo,
    })
    seen = ([""]*post.n) if post.echo else ([post.prompt]*post.n)
    return StreamingResponse(completion_streamer(ticket, post, seen, req["created"]))


async def completion_streamer(ticket: selfhost_req_queue.Ticket, post: NlpCompletion, seen, created_ts, kt, kcomp):
    try:
        while 1:
            try:
                msg = await asyncio.wait_for(ticket.queue.get(), TIMEOUT)
            except asyncio.TimeoutError:
                log("TIMEOUT %s" % ticket.ticket)
                msg = {"status": "error", "human_readable_message": "timeout"}
            not_seen_resp = copy.deepcopy(msg)
            if "choices" in not_seen_resp:
                for i in range(post.n):
                    if not_seen_resp["choices"][i]["text"].startswith(seen[i]):
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
            if msg.get("status", "") != "in_progress":
                break
        if post.stream:
            yield "data: [DONE]" + "\n\n"
        log(red_time(created_ts), "/finished %s" % ticket.ticket)
        ticket.done()
        # fastapi_stats.stats_accum[kt] += msg.get("generated_tokens_n", 0)
        # fastapi_stats.stats_accum[kcomp] += 1
        # fastapi_stats.stats_lists_accum["stat_latency_" + post.model].append(time.time() - created_ts)
    finally:
        if ticket.ticket is not None:
            log("   ***  CANCEL  ***  cancelling %s" % ticket.ticket, red_time(created_ts))
            # fastapi_stats.stats_accum["stat_api_cancelled"] += 1
            # fastapi_stats.stats_accum["stat_m_" + post.model + "_cancelled"] += 1
        ticket.cancelled = True


# class POI(BaseModel):
#     filename: str
#     cursor0: int
#     cursor1: int
#     priority: float


async def static_string_streamer(ticket_id, static_message, account, created_ts):
    yield "data: " + json.dumps({"object": "smc.chat.chunk", "role": "assistant", "delta": static_message, "finish_reason": "END"}) + "\n\n"
    yield "data: [ERROR]" + "\n\n"
    log("  ", red_time(created_ts), "%s chat static message to %s: %s" % (ticket_id, account, static_message))
