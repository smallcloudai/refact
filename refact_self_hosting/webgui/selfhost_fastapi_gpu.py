import time, json, termcolor
import asyncio
from fastapi import APIRouter, Request, Query, Header
from pydantic import BaseModel, Required
from refact_self_hosting.webgui import selfhost_req_queue
from refact_self_hosting.webgui.selfhost_webutils import log
from typing import Dict, List, Optional, Any


router = APIRouter()
ENGINE_WAIT_TIMEOUT = 10


class EngineDescription(BaseModel):
    infmod_guid: str
    B: int = Query(default=0, ge=1, le=64)
    model: str = Query(default=Required, regex="^[a-z/A-Z0-9_\.]+$")
    engine_started_ts: int
    max_thinking_time: int


def red_time(base_ts):
    if base_ts == 0:
        return "???ms"
    return termcolor.colored("%0.1fms" % (1000*(time.time() - base_ts)), "red")


@router.post("/completions-wait-batch")
async def nlp_wait_batch(
    description: EngineDescription,
    request: Request,
    authorization: str = Header(None),
):
    # ip = request.client.host
    # ac_dict = await lookup_bearer(authorization, ip=ip, infserver_mode=True)
    infeng_guid = description.infmod_guid
    model_queue = selfhost_req_queue.global_user2gpu_queue[description.model]
    user_req: Optional[selfhost_req_queue.CompletionRequest] = None
    user_reqs = []
    t0 = time.time()
    for b in range(description.B):
        try:
            if len(user_reqs) == 0:
                time_passed = time.time() - t0
                user_req = await asyncio.wait_for(model_queue.get(), timeout=max(0, ENGINE_WAIT_TIMEOUT - time_passed))
            else:
                user_req = model_queue.get_nowait()
            if user_req.cancelled:
                log(red_time(user_req.call.get("created", 0)), "cancelled %s, drop" % user_req.call["call_id"])
                continue
            user_reqs.append(user_req)
        except (asyncio.TimeoutError, asyncio.queues.QueueEmpty):
            break
    if len(user_reqs) == 0:
        how_busy = 1
        return {"retcode": "WAIT"}
    t1 = time.time()
    how_busy = 1 - (t1 - t0) / ENGINE_WAIT_TIMEOUT
    log("wait_batch batch %i/%i => %s" % (len(user_reqs), description.B, description.infmod_guid))
    return {
        "retcode": "OK",
        "batch": [x.call for x in user_reqs],
    }


class HeadMidTail(BaseModel):
    head: int
    mid: str
    tail: int


class SubSingleNlpChoice(BaseModel):
    index: int
    files: Optional[Dict[str, str]]
    files_head_mid_tail: Optional[Dict[str, HeadMidTail]]
    role: Optional[str]
    content: Optional[str]
    logprobs: Optional[float]
    finish_reason: Optional[str]


class SingleNlpResponse(BaseModel):
    id: str
    object: str
    choices: List[SubSingleNlpChoice]
    status: str
    more_toplevel_fields: Optional[Dict[str, Any]]
    created: float = 0
    generated_tokens_n: int = 0


class NlpResponse(BaseModel):
    infmod_guid: str
    model_name: str
    ts_arrived: float
    ts_batch_started: float
    ts_prompt: float
    ts_first_token: float
    ts_batch_finished: float
    progress: Dict[str, SingleNlpResponse]
    check_cancelled: List[str]


@router.post("/completion-upload-results")
async def nlp_upload_response(
    nlp_response: NlpResponse,
    request: Request,
    authorization: str = Header(None),
):
    # ip = request.client.host
    # ac_dict = await fastapi_auth.lookup_bearer(authorization, red, ip=ip, infserver_mode=True)
    model_name = nlp_response.model_name
    resp: SingleNlpResponse
    for ticket, resp in nlp_response.progress.items():
        ticket_safe = fu.safe_for_redis(ticket)
        saveto = "call_" + ticket_safe + "_resp"
        subname = "call_" + ticket_safe
        # log(termcolor.colored("save resp infengine=%s %s" % (infeng_guid, saveto), "red"))
        msgj = {
            "id": resp.id,
            "object": resp.object,
            "status": resp.status,
            "created": resp.created,
            "uploaded": time.time(),
            "generated_tokens_n": resp.generated_tokens_n,
            "model": model_name,
            "choices": [],
            **(resp.more_toplevel_fields or {}),
        }
        for x in resp.choices:
            choice = {
                "index": x.index,
                "logprobs": x.logprobs,
                "finish_reason": x.finish_reason,
            }
            if x.files is not None:
                if "text" in x.files:
                    choice["text"] = x.files["text"]
                else:
                    choice["files"] = x.files
            if x.files_head_mid_tail is not None:
                choice["files_head_mid_tail"] = dict()
                for fn in x.files_head_mid_tail.keys():
                    choice["files_head_mid_tail"][fn] = {
                        "head": x.files_head_mid_tail[fn].head,
                        "mid": x.files_head_mid_tail[fn].mid,
                        "tail": x.files_head_mid_tail[fn].tail,
                    }
            if x.role is not None:
                choice["role"] = x.role
                choice["content"] = x.content
            msgj["choices"].append(choice)
        if resp.status == "completed":
            created = resp.created
            log("  ", red_time(resp.created), "%s" % ticket,
                "(arrived to gpu %0.1fms prompt %+0.2fms first %+0.2fms onebyone %+0.2fms/%i)" % (
                    1000*(nlp_response.ts_arrived - created),
                    1000*(nlp_response.ts_prompt - nlp_response.ts_arrived),
                    1000*(nlp_response.ts_first_token - nlp_response.ts_prompt),
                    1000*(nlp_response.ts_batch_finished - nlp_response.ts_first_token),
                    resp.generated_tokens_n,
                ))
        msg: str = json.dumps(msgj)
        await red.setex(saveto, 30, msg)
        await red.publish(subname, msg)
    cancelled_tickets = []
    l = len(nlp_response.check_cancelled)
    if 0 < l <= 32:
        is_canceled = await red.mget(["call_" + fu.safe_for_redis(x) + "_cancelled" for x in nlp_response.check_cancelled])
        cancelled_tickets = [x for x, y in zip(nlp_response.check_cancelled, is_canceled) if y]
    return {"retcode": "OK", "cancelled": cancelled_tickets}

