import os
import time
import termcolor
import asyncio

from fastapi import APIRouter, Query, Request, Header, HTTPException

from self_hosting_machinery.webgui.selfhost_req_queue import Ticket
from self_hosting_machinery.webgui.selfhost_webutils import log
from self_hosting_machinery.webgui.selfhost_queue import InferenceQueue

from pydantic import BaseModel, Required
from typing import Dict, List, Optional, Any


__all__ = ["GPURouter"]


def red_time(base_ts):
    if base_ts == 0:
        return "???ms"
    return termcolor.colored("%0.1fms" % (1000*(time.time() - base_ts)), "red")



verify_api_key = os.environ.get("SMALLCLOUD_API_KEY", "EMPTY")


def verify_bearer(authorization: str):
    if verify_api_key is None:
        return
    # if authorization is None:
    #     raise HTTPException(status_code=401, detail="Missing authorization header")
    # bearer_hdr = authorization.split(" ")
    # if len(bearer_hdr) != 2 or bearer_hdr[0] != "Bearer":
    #     raise HTTPException(status_code=401, detail="Invalid authorization header")
    # api_key = bearer_hdr[1]
    # if api_key != verify_api_key:
    #     raise HTTPException(status_code=401, detail="API key mismatch")


class EngineDescription(BaseModel):
    infmod_guid: str
    B: int = Query(default=0, ge=1, le=64)
    model: str = Query(default=Required, regex="^[a-z/A-Z0-9_\.\-]+$")
    engine_started_ts: int
    max_thinking_time: int


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


class GPURouter(APIRouter):

    def __init__(self,
                 inference_queue: InferenceQueue,
                 id2ticket: Dict[str, Ticket],
                 engine_wait_timeout: int = 10,
                 *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.add_api_route("/completions-wait-batch", self._nlp_wait_batch, methods=["POST"])
        self.add_api_route("/completion-upload-results", self._nlp_upload_response, methods=["POST"])
        self._inference_queue = inference_queue
        self._id2ticket = id2ticket
        self._engine_wait_timeout = engine_wait_timeout

    async def _nlp_wait_batch(self, description: EngineDescription, authorization: str = Header(None)):
        verify_bearer(authorization)
        model_queue = self._inference_queue.model_name_to_queue(None, description.model, no_checks=True)
        user_reqs = []
        t0 = time.time()
        for b in range(description.B):
            try:
                if len(user_reqs) == 0:
                    time_passed = time.time() - t0
                    user_req = await asyncio.wait_for(
                        model_queue.get(), timeout=max(0., self._engine_wait_timeout - time_passed))
                else:
                    user_req = model_queue.get_nowait()
                if user_req.cancelled:
                    log(red_time(user_req.call.get("created", 0)) + " cancelled %s, drop" % user_req.call.get("id", "NO-ID"))
                    continue
                user_reqs.append(user_req)
            except (asyncio.TimeoutError, asyncio.queues.QueueEmpty):
                break
        if len(user_reqs) == 0:
            return {"retcode": "WAIT"}
        log("wait_batch batch %i/%i => %s" % (len(user_reqs), description.B, description.infmod_guid))
        return {
            "retcode": "OK",
            "batch": [x.call for x in user_reqs],
        }

    async def _nlp_upload_response(self, nlp_response: NlpResponse, authorization: str = Header(None)):
        verify_bearer(authorization)
        model_name = nlp_response.model_name
        resp: SingleNlpResponse
        cancelled_tickets = []
        for ticket_id, resp in nlp_response.progress.items():
            ticket = self._id2ticket.get(ticket_id)
            if ticket is None:
                log(red_time(resp.created) + " %s result arrived too late" % ticket_id)
                cancelled_tickets.append(ticket_id)
                continue
            if ticket.cancelled:
                log(red_time(resp.created) + " %s result arrived, but ticket is cancelled" % ticket_id)
                cancelled_tickets.append(ticket_id)
                continue
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
                log(red_time(resp.created) + " %s (arrived to gpu %0.1fms prompt %+0.2fms first %+0.2fms onebyone %+0.2fms/%i)" % (
                        ticket_id,
                        1000*(nlp_response.ts_arrived - created),
                        1000*(nlp_response.ts_prompt - nlp_response.ts_arrived),
                        1000*(nlp_response.ts_first_token - nlp_response.ts_prompt),
                        1000*(nlp_response.ts_batch_finished - nlp_response.ts_first_token),
                        resp.generated_tokens_n,
                    ))
            await ticket.streaming_queue.put(msgj)
        return {"retcode": "OK", "cancelled": cancelled_tickets}
