import time
import json
import copy
import asyncio
import aiofiles
import termcolor
import os
import re
import uuid
import litellm
import traceback

from fastapi import APIRouter, HTTPException, Query, Header
from fastapi.responses import Response, StreamingResponse

from itertools import chain

from refact_utils.scripts import env
from refact_utils.finetune.utils import running_models_and_loras
from refact_utils.third_party.utils.models import available_third_party_models
from refact_utils.third_party.utils.tokenizers import load_tokenizer
from refact_webgui.webgui.selfhost_model_resolve import static_resolve_model
from refact_webgui.webgui.selfhost_queue import Ticket
from refact_webgui.webgui.selfhost_webutils import log
from refact_webgui.webgui.selfhost_queue import InferenceQueue
from refact_webgui.webgui.selfhost_model_assigner import ModelAssigner
from refact_webgui.webgui.selfhost_login import RefactSession

from pathlib import Path
from pydantic import BaseModel
from typing import List, Dict, Union, Optional, Tuple, Any

__all__ = ["BaseCompletionsRouter", "CompletionsRouter"]


def clamp(lower, upper, x):
    return max(lower, min(upper, x))


def red_time(base_ts):
    return termcolor.colored("%0.1fms" % (1000*(time.time() - base_ts)), "red")


class NlpSamplingParams(BaseModel):
    max_tokens: Optional[int] = None
    max_completion_tokens: Optional[int] = 500
    temperature: float = 0.2
    top_p: float = 1.0  # TODO: deprecated field
    top_n: int = 0  # TODO: deprecated field
    stop: Union[List[str], str] = []

    @property
    def actual_max_tokens(self):
        if self.max_tokens is not None:
            return max(1, self.max_tokens)
        else:
            return max(1, self.max_completion_tokens)

    def clamp(self):
        self.temperature = clamp(0, 4, self.temperature)
        self.top_p = clamp(0.0, 1.0, self.top_p)
        self.top_n = clamp(0, 1000, self.top_n)
        return {
            "temperature": self.temperature,
            "top_p": self.top_p,
            "top_n": self.top_n,
            "max_tokens": self.actual_max_tokens,
            "created": time.time(),
            "stop_tokens": self.stop,
        }


class NlpCompletion(NlpSamplingParams):
    model: str = Query(pattern="^[a-z/A-Z0-9_\.\-\:]+$")
    prompt: str
    n: int = 1
    echo: bool = False
    stream: bool = False
    mask_emails: bool = False


class ChatMessage(BaseModel):
    role: str
    content: Union[str, List[Dict]]
    tool_calls: Optional[List[Dict[str, Any]]] = None
    tool_call_id: Optional[str] = None
    thinking_blocks: Optional[List[Dict]] = None


class ChatContext(NlpSamplingParams):
    model: str = Query(pattern="^[a-z/A-Z0-9_\.\-\:]+$")
    messages: List[ChatMessage]
    tools: Optional[List[Dict[str, Any]]] = None
    tool_choice: Optional[str] = None
    stream: Optional[bool] = True
    n: int = 1
    reasoning_effort: Optional[str] = None  # OpenAI style reasoning
    thinking: Optional[Dict] = None  # Anthropic style reasoning


class EmbeddingsStyleOpenAI(BaseModel):
    input: Union[str, List[str]]
    model: str = Query(pattern="^[a-z/A-Z0-9_\.\-]+$")


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


class BaseCompletionsRouter(APIRouter):

    def __init__(self,
                 inference_queue: InferenceQueue,
                 id2ticket: Dict[str, Ticket],
                 model_assigner: ModelAssigner,
                 timeout: int = 30,
                 *args, **kwargs):
        super().__init__(*args, **kwargs)

        self.add_api_route("/refact-caps", self._caps, methods=["GET"])
        self.add_api_route("/v1/completions", self._completions, methods=["POST"])
        self.add_api_route("/v1/embeddings", self._embeddings_style_openai, methods=["POST"])
        self.add_api_route("/v1/chat/completions", self._chat_completions, methods=["POST"])

        self.add_api_route("/tokenizer/{model_name}", self._tokenizer, methods=["GET"])

        self._inference_queue = inference_queue
        self._id2ticket = id2ticket
        self._model_assigner = model_assigner
        self._timeout = timeout

    @property
    def _caps_version(self) -> int:
        cfg_active_lora_mtime = int(os.path.getmtime(env.CONFIG_ACTIVE_LORA)) if os.path.isfile(env.CONFIG_ACTIVE_LORA) else 0
        cfg_third_party_mtime = int(os.path.getmtime(env.CONFIG_THIRD_PARTY_MODELS)) if os.path.isfile(env.CONFIG_THIRD_PARTY_MODELS) else 0
        return max(self._model_assigner.config_inference_mtime(), cfg_active_lora_mtime, cfg_third_party_mtime)

    async def _account_from_bearer(self, authorization: str) -> str:
        raise NotImplementedError()

    async def _caps(self, authorization: str = Header(None), user_agent: str = Header(None)):
        client_version = self._parse_client_version(user_agent)
        data = self._caps_data()
        if client_version is not None and client_version < (0, 10, 15):
            log(f"{user_agent} is deprecated, fallback to old caps format. Please upgrade client's plugin.")
            data = self._to_deprecated_caps_format(data)
        return Response(content=json.dumps(data, indent=4), media_type="application/json")

    def _caps_data(self):
        # NOTE: we need completely rewrite all about running models
        running_models = running_models_and_loras(self._model_assigner)

        def _get_base_model_info(model_name: str) -> str:
            return model_name.split(":")[0]

        def _select_default_model(models: List[str]) -> str:
            if not models:
                return ""
            default_model = models[0]
            default_model_loras = [
                model_name for model_name in models
                if model_name.startswith(f"{default_model}:")
            ]
            if default_model_loras:
                return default_model_loras[0]
            return default_model

        # completion models
        completion_models = {}
        for model_name in running_models.get("completion", []):
            base_model_name = _get_base_model_info(model_name)
            if model_info := self._model_assigner.models_db.get(base_model_name):
                completion_models[model_name] = self._model_assigner.to_completion_model_record(base_model_name, model_info)
            elif model := available_third_party_models().get(model_name):
                completion_models[model_name] = model.to_completion_model_record()
            else:
                log(f"completion model `{model_name}` is listed as running but not found in configs, skip")
        completion_default_model = _select_default_model(list(completion_models.keys()))

        # chat models
        chat_models = {}
        for model_name in running_models.get("chat", []):
            base_model_name = _get_base_model_info(model_name)
            if model_info := self._model_assigner.models_db.get(base_model_name):
                chat_models[model_name] = self._model_assigner.to_chat_model_record(base_model_name, model_info)
            elif model := available_third_party_models().get(model_name):
                chat_models[model_name] = model.to_chat_model_record()
            else:
                log(f"chat model `{model_name}` is listed as running but not found in configs, skip")
        chat_default_model = _select_default_model(list(chat_models.keys()))

        # embedding models
        embedding_models = {}
        for model_name in running_models.get("embedding", []):
            if model_info := self._model_assigner.models_db.get(_get_base_model_info(model_name)):
                embedding_models[model_name] = {
                    "n_ctx": model_info["T"],
                    "size": model_info["size"],
                }
            else:
                log(f"embedding model `{model_name}` is listed as running but not found in configs, skip")
        embedding_default_model = _select_default_model(list(embedding_models.keys()))

        # tokenizer endpoints
        tokenizer_endpoints = {}
        for model_list in running_models.values():
            for model_name in model_list:
                tokenizer_endpoints[model_name] = "/tokenizer/" + _get_base_model_info(model_name).replace("/", "--")

        data = {
            "cloud_name": "Refact Self-Hosted",

            "completion": {
                "endpoint": "/v1/completions",
                "models": completion_models,
                "default_model": completion_default_model,
                "default_multiline_model": completion_default_model,
            },

            "chat": {
                "endpoint": "/v1/chat/completions",
                "models": chat_models,
                "default_model": chat_default_model,
            },

            "embedding": {
                "endpoint": "v1/embeddings",
                "models": embedding_models,
                "default_model": embedding_default_model,
            },

            "telemetry_endpoints": {
                "telemetry_basic_endpoint": "/stats/telemetry-basic",
                "telemetry_corrected_snippets_endpoint": "/stats/telemetry-snippets",
                "telemetry_basic_retrieve_my_own_endpoint": "/stats/rh-stats",
            },

            "tokenizer_endpoints": tokenizer_endpoints,

            "caps_version": self._caps_version,
        }

        return data

    @staticmethod
    def _parse_client_version(user_agent: str = Header(None)) -> Optional[Tuple[int, int, int]]:
        if not isinstance(user_agent, str):
            log(f"unknown client version `{user_agent}`")
            return None
        m = re.match(r"^refact-lsp (\d+)\.(\d+)\.(\d+)$", user_agent)
        if not m:
            log(f"can't parse client version `{user_agent}`")
            return None
        major, minor, patch = map(int, m.groups())
        log(f"client version {major}.{minor}.{patch}")
        return major, minor, patch

    @staticmethod
    def _to_deprecated_caps_format(data: Dict[str, Any]):
        models_dict_patch = {}
        for model_name, model_record in chain(
                data["completion"]["models"].items(),
                data["completion"]["models"].items(),
        ):
            dict_patch = {}
            if n_ctx := model_record.get("n_ctx"):
                dict_patch["n_ctx"] = n_ctx
            if supports_tools := model_record.get("supports_tools"):
                dict_patch["supports_tools"] = supports_tools
            if dict_patch:
                models_dict_patch[model_name] = dict_patch
        return {
            "cloud_name": data["cloud_name"],
            "endpoint_template": data["completion"]["endpoint"],
            "endpoint_chat_passthrough": data["chat"]["endpoint"],
            "endpoint_style": "openai",
            "telemetry_basic_dest": data["telemetry_endpoints"]["telemetry_basic_endpoint"],
            "telemetry_corrected_snippets_dest": data["telemetry_endpoints"]["telemetry_corrected_snippets_endpoint"],
            "telemetry_basic_retrieve_my_own": data["telemetry_endpoints"]["telemetry_basic_retrieve_my_own_endpoint"],
            "running_models": list(data["completion"]["models"].keys()) + list(data["chat"]["models"].keys()),
            "code_completion_default_model": data["completion"]["default_model"],
            "multiline_code_completion_default_model": data["completion"]["default_multiline_model"],
            "code_chat_default_model": data["chat"]["default_model"],
            "models_dict_patch": models_dict_patch,
            "default_embeddings_model": data["embedding"]["default_model"],
            "endpoint_embeddings_template": "v1/embeddings",
            "endpoint_embeddings_style": "openai",
            "size_embeddings": 768,
            "tokenizer_path_template": "/tokenizer/$MODEL",
            "tokenizer_rewrite_path": {
                model_name: tokenizer_url.replace("/tokenizer/", "")
                for model_name, tokenizer_url in data["tokenizer_endpoints"].items()
            },
            "caps_version": data["caps_version"],
        }

    async def _local_tokenizer(self, model_path: str) -> str:
        model_dir = Path(env.DIR_WEIGHTS) / f"models--{model_path.replace('/', '--')}"
        tokenizer_paths = list(sorted(model_dir.rglob("tokenizer.json"), key=lambda p: p.stat().st_ctime))
        if not tokenizer_paths:
            raise HTTPException(404, detail=f"tokenizer.json for {model_path} does not exist")

        data = ""
        async with aiofiles.open(tokenizer_paths[-1], mode='r') as f:
            while True:
                if not (chunk := await f.read(1024 * 1024)):
                    break
                data += chunk

        return data

    async def _tokenizer(self, model_name: str):
        model_name = model_name.replace("--", "/")
        try:
            if model_name in self._model_assigner.models_db:
                model_path = self._model_assigner.models_db[model_name]["model_path"]
                data = await self._local_tokenizer(model_path)
            elif model := available_third_party_models().get(model_name):
                data = await load_tokenizer(model.tokenizer_id)
            else:
                raise RuntimeError(f"model `{model_name}` is not serving")
            return Response(content=data, media_type='application/json')
        except RuntimeError as e:
            raise HTTPException(404, detail=str(e))

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
        caps_version = self._caps_version  # use mtime as a version, if that changes the client will know to refresh caps

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
        # TODO: we'll implement caps_version logic later

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

    async def _chat_completions(self, post: ChatContext, authorization: str = Header(None)):
        created_ts = time.time()
        request_id = f"chat-comp-{str(uuid.uuid4()).replace('-', '')[0:12]}"

        _account = await self._account_from_bearer(authorization)
        caps_version = self._caps_version

        messages = []
        # drop empty optional fields
        for m in (i.dict() for i in post.messages):
            if "tool_calls" in m and not m["tool_calls"]:
                del m["tool_calls"]
            if "thinking_blocks" in m and not m["thinking_blocks"]:
                del m["thinking_blocks"]
            messages.append(m)

        prefix, postfix = "data: ", "\n\n"

        def _patch_caps_version(data: Dict) -> Dict:
            return {
                **data,
                "caps_version": caps_version,
            }

        def _wrap_output(output: str) -> str:
            return prefix + output + postfix

        model_config = available_third_party_models().get(post.model)
        if model_config:
            log(f"{request_id}: resolve {post.model} -> {model_config.model_id}")
        else:
            err_message = f"model {post.model} is not running on server"
            log(f"{request_id}: {err_message}")
            raise HTTPException(status_code=400, detail=err_message)

        messages_to_count = [{k: v for k, v in m.items() if k not in ["thinking_blocks"]} for m in messages]
        prompt_tokens_n = litellm.token_counter(model_config.model_id, messages=messages_to_count, tools=post.tools)

        max_tokens = min(model_config.max_tokens, post.actual_max_tokens)
        completion_kwargs = {
            "model": model_config.model_id,
            "api_base": model_config.api_base,
            "api_key": model_config.api_key,
            "messages": messages,
            "temperature": post.temperature,
            "top_p": post.top_p,
            "max_tokens": max_tokens,
            "tools": post.tools,
            "tool_choice": post.tool_choice,
            "stop": post.stop if post.stop else None,
            "n": post.n,
            "extra_headers": model_config.extra_headers if model_config.extra_headers else None,
        }

        if post.reasoning_effort or post.thinking:
            del completion_kwargs["temperature"]
            del completion_kwargs["top_p"]

        if post.reasoning_effort:
            completion_kwargs["reasoning_effort"] = post.reasoning_effort
        if post.thinking:
            completion_kwargs["thinking"] = post.thinking

        async def litellm_streamer():
            generated_tokens_n = 0
            try:
                response = await litellm.acompletion(
                    **completion_kwargs, stream=True,
                )
                finish_reason = None
                async for model_response in response:
                    try:
                        data = model_response.dict()
                        choice0 = data["choices"][0]
                        finish_reason = choice0["finish_reason"]
                        if delta := choice0.get("delta"):
                            if text := delta.get("content"):
                                generated_tokens_n += litellm.token_counter(model_config.model_id, text=text)

                    except json.JSONDecodeError:
                        data = {"choices": [{"finish_reason": finish_reason}]}
                    yield _wrap_output(json.dumps(_patch_caps_version(data)))

                final_msg: Dict[str, Any] = {"choices": []}
                usage_dict = model_config.compose_usage_dict(prompt_tokens_n, generated_tokens_n)
                final_msg.update(usage_dict)
                yield _wrap_output(json.dumps(_patch_caps_version(final_msg)))

                # NOTE: DONE needed by refact-lsp server
                yield _wrap_output("[DONE]")
                log(f"{request_id} /finished in {red_time(created_ts)}")
            except BaseException as e:
                err_msg = f"litellm error (streaming): {e}"
                log(f"{request_id} /error: {err_msg}, {red_time(created_ts)}")
                yield _wrap_output(json.dumps(_patch_caps_version({"error": err_msg})))

        async def litellm_non_streamer():
            generated_tokens_n = 0
            try:
                model_response = await litellm.acompletion(
                    **completion_kwargs, stream=False,
                )
                finish_reason = None
                try:
                    data = model_response.dict()
                    for choice in data.get("choices", []):
                        if text := choice.get("message", {}).get("content"):
                            generated_tokens_n += litellm.token_counter(model_config.model_id, text=text)
                        finish_reason = choice.get("finish_reason")
                    usage_dict = model_config.compose_usage_dict(prompt_tokens_n, generated_tokens_n)
                    data.update(usage_dict)
                except json.JSONDecodeError:
                    data = {"choices": [{"finish_reason": finish_reason}]}
                yield json.dumps(_patch_caps_version(data))
                log(f"{request_id} /finished in {red_time(created_ts)}")
            except BaseException as e:
                err_msg = f"litellm error (no streaming): {e}"
                log(f"{request_id} /error: {err_msg}, {red_time(created_ts)}")
                yield json.dumps(_patch_caps_version({"error": err_msg}))

        response_streamer = litellm_streamer() if post.stream else litellm_non_streamer()

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
