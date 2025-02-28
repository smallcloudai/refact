import time
import json
import copy
import asyncio
import aiohttp
import aiofiles
import termcolor
import os
import re
import litellm
import traceback

from fastapi import APIRouter, HTTPException, Query, Header
from fastapi.responses import Response, StreamingResponse

from refact_utils.scripts import env
from refact_utils.finetune.utils import running_models_and_loras
from refact_webgui.webgui.selfhost_model_resolve import resolve_model_context_size
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
    # TODO: validate using pydantic
    tool_calls: Optional[List[Dict[str, Any]]] = None
    tool_call_id: Optional[str] = None


class ChatContext(NlpSamplingParams):
    model: str = Query(pattern="^[a-z/A-Z0-9_\.\-\:]+$")
    messages: List[ChatMessage]
    # TODO: validate using pydantic
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

        # deprecated APIs
        self.add_api_route("/coding_assistant_caps.json", self._coding_assistant_caps, methods=["GET"])
        self.add_api_route("/v1/login", self._login, methods=["GET"])

        # API for LSP server
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
        return max(self._model_assigner.config_inference_mtime(), cfg_active_lora_mtime)

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

        litellm.modify_params = True  # NOTE: for Anthropic API
        litellm.drop_params = True    # NOTE: for OpenAI API
        _integrations_env_setup("OPENAI_API_KEY", "openai_api_key", "openai_api_enable")
        _integrations_env_setup("ANTHROPIC_API_KEY", "anthropic_api_key", "anthropic_api_enable")
        _integrations_env_setup("GROQ_API_KEY", "groq_api_key", "groq_api_enable")
        _integrations_env_setup("CEREBRAS_API_KEY", "cerebras_api_key", "cerebras_api_enable")
        _integrations_env_setup("GEMINI_API_KEY", "gemini_api_key", "gemini_api_enable")
        _integrations_env_setup("XAI_API_KEY", "xai_api_key", "xai_api_enable")
        _integrations_env_setup("DEEPSEEK_API_KEY", "deepseek_api_key", "deepseek_api_enable")

    def _models_available_dict_rewrite(self, models_available: List[str]) -> Dict[str, Any]:
        rewrite_dict = {}
        for model in models_available:
            d = {}
            if n_ctx := resolve_model_context_size(model, self._model_assigner):
                d["n_ctx"] = n_ctx
            if "tools" in self._model_assigner.models_db_with_passthrough.get(model, {}).get("filter_caps", []):
                d["supports_tools"] = True

            rewrite_dict[model] = d
        return rewrite_dict

    def _caps_base_data(self) -> Dict[str, Any]:
        running = running_models_and_loras(self._model_assigner)
        models_available = self._inference_queue.models_available(force_read=True)
        code_completion_default_model, _ = self._inference_queue.completion_model()
        multiline_code_completion_default_model, _ = self._inference_queue.multiline_completion_default_model()
        code_chat_default_model = ""
        embeddings_default_model = ""
        for model_name in models_available:
            if "chat" in self._model_assigner.models_db.get(model_name, {}).get("filter_caps", []) or model_name in litellm.model_list:
                if not code_chat_default_model:
                    code_chat_default_model = model_name
            if "embeddings" in self._model_assigner.models_db.get(model_name, {}).get("filter_caps", []):
                if not embeddings_default_model:
                    embeddings_default_model = model_name
        data = {
            "cloud_name": "Refact Self-Hosted",
            "endpoint_template": "/v1/completions",
            "endpoint_chat_passthrough": "/v1/chat/completions",
            "endpoint_style": "openai",
            "telemetry_basic_dest": "/stats/telemetry-basic",
            "telemetry_corrected_snippets_dest": "/stats/telemetry-snippets",
            "telemetry_basic_retrieve_my_own": "/stats/rh-stats",
            "running_models": list(set(r for r in [*running['completion'], *running['chat']])),
            "code_completion_default_model": code_completion_default_model,
            "multiline_code_completion_default_model": multiline_code_completion_default_model,
            "code_chat_default_model": code_chat_default_model,
            "models_dict_patch": self._models_available_dict_rewrite(models_available),

            "default_embeddings_model": embeddings_default_model,
            "endpoint_embeddings_template": "v1/embeddings",
            "endpoint_embeddings_style": "openai",
            "size_embeddings": 768,

            "tokenizer_path_template": "/tokenizer/$MODEL",
            "tokenizer_rewrite_path": {model: model.replace("/", "--") for model in models_available},
            "caps_version": self._caps_version,
        }

        return data

    async def _coding_assistant_caps(self):
        log(f"Your refact-lsp version is deprecated, finetune is unavailable. Please update your plugin.")
        return Response(content=json.dumps(self._caps_base_data(), indent=4), media_type="application/json")

    async def _caps(self, authorization: str = Header(None), user_agent: str = Header(None)):
        if isinstance(user_agent, str):
            m = re.match(r"^refact-lsp (\d+)\.(\d+)\.(\d+)$", user_agent)
            if m:
                major, minor, patch = map(int, m.groups())
                log("user version %d.%d.%d" % (major, minor, patch))
        data = self._caps_base_data()
        running = running_models_and_loras(self._model_assigner)

        def _select_default_lora_if_exists(model_name: str, running_models: List[str]):
            model_variants = [r for r in running_models if r.split(":")[0] == model_name and r != model_name]
            return model_variants[0] if model_variants else model_name

        data["code_completion_default_model"] = _select_default_lora_if_exists(
            data["code_completion_default_model"],
            running['completion'],
        )
        data["multiline_code_completion_default_model"] = _select_default_lora_if_exists(
            data["multiline_code_completion_default_model"],
            running['completion'],
        )
        data["code_chat_default_model"] = _select_default_lora_if_exists(
            data["code_chat_default_model"],
            running['chat'],
        )

        return Response(content=json.dumps(data, indent=4), media_type="application/json")

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

    async def _passthrough_tokenizer(self, model_path: str) -> str:
        try:
            async with aiohttp.ClientSession() as session:
                tokenizer_url = f"https://huggingface.co/{model_path}/resolve/main/tokenizer.json"
                async with session.get(tokenizer_url) as resp:
                    return await resp.text()
        except:
            raise HTTPException(404, detail=f"can't load tokenizer.json for passthrough {model_path}")

    async def _tokenizer(self, model_name: str):
        model_name = model_name.replace("--", "/")
        if model_name in self._model_assigner.models_db:
            model_path = self._model_assigner.models_db[model_name]["model_path"]
            data = await self._local_tokenizer(model_path)
        elif model_name in self._model_assigner.passthrough_mini_db:
            model_path = self._model_assigner.passthrough_mini_db[model_name]["tokenizer_path"]
            data = await self._passthrough_tokenizer(model_path)
        else:
            raise HTTPException(404, detail=f"model '{model_name}' does not exists in db")
        return Response(content=data, media_type='application/json')

    async def _login(self, authorization: str = Header(None)) -> Dict:
        account = await self._account_from_bearer(authorization)
        return {
            "account": account,
            "retcode": "OK",
            "chat-v1-style": 1,
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
        def compose_usage_dict(model_dict, prompt_tokens_n, generated_tokens_n) -> Dict[str, Any]:
            usage_dict = dict()
            usage_dict["pp1000t_prompt"] = model_dict.get("pp1000t_prompt", 0)
            usage_dict["pp1000t_generated"] = model_dict.get("pp1000t_generated", 0)
            usage_dict["metering_prompt_tokens_n"] = prompt_tokens_n
            usage_dict["metering_generated_tokens_n"] = generated_tokens_n
            return usage_dict

        _account = await self._account_from_bearer(authorization)
        caps_version = self._caps_version

        messages = []
        for m in (i.dict() for i in post.messages):
            # drop tool_calls if empty, otherwise litellm tokenizing won't work
            if "tool_calls" in m and not m["tool_calls"]:
                del m["tool_calls"]
            messages.append(m)

        prefix, postfix = "data: ", "\n\n"

        def _patch_caps_version(data: Dict) -> Dict:
            return {
                **data,
                "caps_version": caps_version,
            }

        def _wrap_output(output: str) -> str:
            return prefix + output + postfix

        model_dict = self._model_assigner.models_db_with_passthrough.get(post.model, {})
        assert model_dict.get('backend') == 'litellm'

        model_name = model_dict.get('resolve_as', post.model)
        if model_name not in litellm.model_list:
            log(f"warning: requested model {model_name} is not in the litellm.model_list (this might not be the issue for some providers)")
        log(f"chat/completions: model resolve {post.model} -> {model_name}")
        prompt_tokens_n = litellm.token_counter(model_name, messages=messages)
        if post.tools:
            prompt_tokens_n += litellm.token_counter(model_name, text=json.dumps(post.tools))

        max_tokens = min(model_dict.get('T_out', post.actual_max_tokens), post.actual_max_tokens)
        completion_kwargs = {
            "model": model_name,
            "messages": messages,
            "temperature": post.temperature,
            "top_p": post.top_p,
            "max_tokens": max_tokens,
            "tools": post.tools,
            "tool_choice": post.tool_choice,
            "stop": post.stop if post.stop else None,
            "n": post.n,
        }

        if post.reasoning_effort or post.thinking:
            del completion_kwargs["temperature"]
            del completion_kwargs["top_p"]

        if post.reasoning_effort:
            completion_kwargs["reasoning_effort"] = post.reasoning_effort
        if post.thinking:
            completion_kwargs["thinking"] = post.thinking

        log(f"{completion_kwargs}")
        if model_name.startswith("claude"):
            with open("completion_kwargs.json", "w") as f:
                json.dump(completion_kwargs, f)

        async def litellm_streamer():
            generated_tokens_n = 0
            try:
                self._integrations_env_setup()
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
                                generated_tokens_n += litellm.token_counter(model_name, text=text)

                    except json.JSONDecodeError:
                        data = {"choices": [{"finish_reason": finish_reason}]}
                    yield _wrap_output(json.dumps(_patch_caps_version(data)))

                final_msg = {"choices": []}
                usage_dict = compose_usage_dict(model_dict, prompt_tokens_n, generated_tokens_n)
                final_msg.update(usage_dict)
                yield _wrap_output(json.dumps(_patch_caps_version(final_msg)))

                # NOTE: DONE needed by refact-lsp server
                yield _wrap_output("[DONE]")
            except BaseException as e:
                err_msg = f"litellm error (1): {e}"
                log(err_msg)
                yield _wrap_output(json.dumps(_patch_caps_version({"error": err_msg})))

        async def litellm_non_streamer():
            generated_tokens_n = 0
            try:
                self._integrations_env_setup()
                model_response = await litellm.acompletion(
                    **completion_kwargs, stream=False,
                )
                finish_reason = None
                try:
                    data = model_response.dict()
                    for choice in data.get("choices", []):
                        if text := choice.get("message", {}).get("content"):
                            generated_tokens_n += litellm.token_counter(model_name, text=text)
                        finish_reason = choice.get("finish_reason")
                    usage_dict = compose_usage_dict(model_dict, prompt_tokens_n, generated_tokens_n)
                    data.update(usage_dict)
                except json.JSONDecodeError:
                    data = {"choices": [{"finish_reason": finish_reason}]}
                yield json.dumps(_patch_caps_version(data))
            except BaseException as e:
                err_msg = f"litellm error (2): {e}"
                log(err_msg)
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
