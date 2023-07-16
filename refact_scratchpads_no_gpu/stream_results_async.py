import os, sys, json, re, time, datetime, termcolor, multiprocessing, copy, queue
import aiohttp
import asyncio
from refact_scratchpads_no_gpu import stream_results
from typing import Dict, Any, List, Optional, Set


validate_description_dict = stream_results.validate_description_dict
logger = stream_results.logger


WAIT_TIMEOUT = 15


def infserver_async_session() -> aiohttp.ClientSession:
    if "SMALLCLOUD_API_KEY" not in os.environ:
        raise ValueError("Please set SMALLCLOUD_API_KEY environment variable, make sure you have rights to host a model.")
    s = aiohttp.ClientSession()
    s.headers.update({
        "Authorization": "Bearer %s" % os.environ["SMALLCLOUD_API_KEY"],
    })
    return s


async def completions_wait_batch(
    aio_session: aiohttp.ClientSession,
    my_desc,
    verbose=False
):
    txt = ""
    j = None
    for attempt in range(5):
        t0 = time.time()
        url = stream_results.url_get_the_best() + "completions-wait-batch"
        try:
            async with aio_session.post(url, json=my_desc, timeout=WAIT_TIMEOUT) as resp:
                txt = await resp.text()
                j = await resp.json()
        except asyncio.TimeoutError:
            t1 = time.time()
            logger.warning("%s %0.1fms %s %s" % (datetime.datetime.now().strftime("%H:%M:%S.%f"), 1000*(t1 - t0), url, termcolor.colored("TIMEOUT", "green")))
            stream_results.url_complain_doesnt_work()
            continue
        except aiohttp.ClientError as e:
            logger.warning("%s fetch batch failed: %s %s\nServer response was: \"%s\"" % (url, str(type(e)), str(e), txt[:150] if txt else "no response"))
            stream_results.url_complain_doesnt_work()
            continue
        if "retcode" not in j:
            logger.warning("%s unrecognized json: %s" % (url, txt[:150]))
            stream_results.url_complain_doesnt_work()
            continue
        break
    if j is None:
        return "ERROR", []
    t1 = time.time()
    hms = datetime.datetime.now().strftime("%H:%M:%S.%f")
    logger.info("%s %0.1fms %s %s" % (hms, 1000*(t1 - t0), url, termcolor.colored(j.get("retcode", "no retcode"), "green")))
    if verbose:
        logger.info("%s %s" % (url, json.dumps(j, indent=4)))
    return j.get("retcode", "ERROR"), j.get("batch", [])


head_and_tail = stream_results.head_and_tail


class UploadAsync:
    def __init__(self):
        self.aio_session = infserver_async_session()
        self.upload_q = asyncio.Queue()
        self.cancelled_q = asyncio.Queue()
        self._cancelled: Set[str] = set()

    def cancelled_reset(self):
        while not self.cancelled_q.empty():
            self._cancelled.add(self.cancelled_q.get_nowait())
        self._cancelled = set()

    def check_cancelled(self):
        while not self.cancelled_q.empty():
            self._cancelled.add(self.cancelled_q.get_nowait())
        return self._cancelled

    def upload_result(
        self,
        description_dict: Dict[str, Any],
        original_batch: Dict[str, Any],
        *,
        status: str,                  # "in_progress", "completed"
        idx_updated: List[int],       # batch indexes where you have progress
        files: List[Dict[str, str]],  # updated text in those indexes
        finish_reason: List[str],     # empty if not finished yet
        tokens: Optional[List[int]] = None,
        more_toplevel_fields: Optional[List[Dict[str, Any]]] = None,
        generated_tokens_n: Optional[List[int]] = None,
        ts_arrived: float,
        ts_batch_started: float,
        ts_prompt: float,
        ts_first_token: float,
        ts_batch_finished: float,
    ):
        upload_dict = copy.deepcopy(description_dict)
        upload_dict["ts_arrived"] = ts_arrived
        upload_dict["ts_batch_started"] = ts_batch_started
        upload_dict["ts_prompt"] = ts_prompt
        upload_dict["ts_first_token"] = ts_first_token
        upload_dict["ts_batch_finished"] = ts_batch_finished
        progress = dict()
        for i, b in enumerate(idx_updated):
            tmp = {
                "id": original_batch[b]["id"],
                "stream": original_batch[b]["stream"],
                "object": "text_completion",
                "choices": [
                    {
                        "index": 0,
                        # "files": files[i],
                        # "tokens": ([int(t) for t in tokens[b]] if tokens is not None else None),
                        "logprobs": None,
                        "finish_reason": finish_reason[i]
                    },
                ],
                "status": status,
                "created": original_batch[b]["created"],
                "more_toplevel_fields": (more_toplevel_fields[i] if more_toplevel_fields is not None else dict()),
                "generated_tokens_n": (generated_tokens_n[i] if generated_tokens_n is not None else 0),
            }
            if "chat__role" in files[i]:
                # deprecated, "chat__messages" is the new way
                tmp["choices"][0]["role"] = files[i]["chat__role"]
                tmp["choices"][0]["content"] = files[i]["chat__content"]
            elif "chat__messages" in files[i]:
                tmp["choices"][0]["messages"] = files[i]["chat__messages"]
            else:
                # normal
                tmp["choices"][0]["files"] = files[i]
            if "sources" in original_batch[b]:
                tmp["orig_files"] = original_batch[b]["sources"]
            progress[original_batch[b]["id"]] = tmp
        upload_dict["progress"] = progress
        upload_dict["check_cancelled"] = [call["id"] for call in original_batch]
        upload_dict["model_name"] = description_dict["model"]
        self.upload_q.put_nowait(copy.deepcopy(upload_dict))

    async def upload_results_coroutine(self):
        exit_flag = False
        while not exit_flag:
            try:
                upload_dict = await asyncio.wait_for(self.upload_q.get(), timeout=600)
            except asyncio.TimeoutError:
                logger.warning("%s %s" % (datetime.datetime.now().strftime("%H:%M:%S.%f"), termcolor.colored("upload_results_loop cancelled", "red")))
                exit_flag = True
                continue
            if "exit" in upload_dict:
                exit_flag = True
                break
            if "progress" not in upload_dict:
                continue
            t1 = time.time()
            while 1:
                if upload_dict.get("ts_batch_finished", 0) > 0:
                    # Send ASAP
                    break
                maybe_pile_up = self.upload_q.get_nowait() if not self.upload_q.empty() else None
                if maybe_pile_up is None:
                    if time.time() < t1 + 0.3:
                        # Normally send every ~0.5 seconds
                        time.sleep(0.1)
                        continue
                    else:
                        break
                if "exit" in maybe_pile_up:
                    exit_flag = True
                if "progress" in maybe_pile_up:
                    upload_dict["progress"].update(maybe_pile_up["progress"])
                    upload_dict["ts_batch_finished"] = maybe_pile_up["ts_batch_finished"]
            # Remove head and tail if streaming, "files" becomes "files_head_mid_tail"
            for k, progress_dict in upload_dict["progress"].items():
                stream = progress_dict["stream"]
                have_orig_files = "orig_files" in progress_dict
                if have_orig_files:
                    orig_files = progress_dict.pop("orig_files")
                if not stream or not have_orig_files:
                    continue
                stream_files = dict()
                for choice in progress_dict["choices"]:
                    files = choice.pop("files")
                    for k in files.keys():
                        orig = orig_files[k]
                        if not orig.endswith("\n"):
                            orig += "\n"
                        head, tail = head_and_tail(orig, files[k])
                        stream_files[k] = {
                            "head": head,
                            "mid": files[k][head:-tail],
                            "tail": tail,
                        }
                    choice["files_head_mid_tail"] = stream_files
            t2 = time.time()
            txt = ""
            for _attempt in range(5):
                j = dict()
                try:
                    url = stream_results.url_get_the_best() + "completion-upload-results"
                    async with self.aio_session.post(url, json=upload_dict, timeout=2) as resp:
                        txt = await resp.text()
                        j = await resp.json()
                except asyncio.exceptions.TimeoutError as e:
                    t1 = time.time()
                    logger.warning("%s %0.1fms %s %s" % (datetime.datetime.now().strftime("%H:%M:%S.%f"), 1000*(time.time() - t2), url, termcolor.colored("TIMEOUT", "green")))
                    stream_results.url_complain_doesnt_work()
                    continue
                except aiohttp.ClientError as e:
                    logger.warning("%s post response failed: %s\nServer response was: \"%s\"" % (url, str(e), txt[:150] if txt else "no response"))
                    stream_results.url_complain_doesnt_work()
                    continue
                if "retcode" not in j:
                    logger.warning("%s unrecognied json: %s" % (url, txt[:150]))
                    stream_results.url_complain_doesnt_work()
                    continue
                break
            t3 = time.time()
            cancelled_n = 0
            if "cancelled" in j:
                for can in j["cancelled"]:
                    self.cancelled_q.put_nowait(can)
                    cancelled_n += 1
            logger.info("%s %s %s %s %i uploaded, %i cancelled" % (datetime.datetime.now().strftime("%H:%M:%S.%f"),
                termcolor.colored("%0.1fms" % (1000*(t3 - t2),), "green"),
                url,
                j.get("retcode", "FAIL"),
                len(upload_dict["progress"]),
                cancelled_n,
                ))
            if j.get("retcode", "FAIL") != "OK":
                logger.warning("Server returned:", txt[:150])

    async def shutdown_coroutine(self):
        await self.upload_q.put({"exit": True})

    async def close_session(self):
        await self.aio_session.close()

    async def keepalive(self):
        await self.upload_q.put(dict(keepalive=1))
