import logging
import os
import time
from itertools import chain

import torch
import traceback
import termcolor

from transformers import AutoModelForCausalLM
from transformers import AutoTokenizer
from transformers import StoppingCriteria
from transformers import StoppingCriteriaList
from transformers.generation.streamers import TextStreamer

from refact_utils.huggingface.utils import huggingface_hub_token
from self_hosting_machinery.inference.scratchpad_hf import ScratchpadHuggingfaceBase
from self_hosting_machinery.inference.scratchpad_hf import ScratchpadHuggingfaceCompletion
from self_hosting_machinery.inference import InferenceBase
from self_hosting_machinery.inference.stream_results import UploadProxy

from refact_utils.scripts import env

from typing import Dict, Any, Optional

from self_hosting_machinery.inference.inference_base import find_param_by_name
from self_hosting_machinery.inference.lora_loader_mixin import LoraLoaderMixin

quit_flag = False
DEBUG = int(os.environ.get("DEBUG", "0"))


class CancellationStoppingCriteria(StoppingCriteria):

    def __init__(self, scratchpad: ScratchpadHuggingfaceBase, request_id: str, upload_proxy: UploadProxy):
        StoppingCriteria.__init__(self)
        self.scratchpad = scratchpad
        self.upload_proxy = upload_proxy
        self.request_id = request_id

    def __call__(self, input_ids: torch.LongTensor, scores: torch.FloatTensor, **kwargs) -> bool:
        if self.request_id in self.upload_proxy.check_cancelled():
            self.scratchpad.finish_reason = "cancelled"
            return True
        return False


class FeedScratchoadCriteria(StoppingCriteria):

    def __init__(self, tokenizer, t0: float, scratchpad: ScratchpadHuggingfaceBase):
        StoppingCriteria.__init__(self)
        self.tokenizer = tokenizer
        self.scratchpad = scratchpad
        self.t0 = t0

    def __call__(self, input_ids: torch.LongTensor, scores: torch.FloatTensor, **kwargs) -> bool:
        token = input_ids[0][-1]
        if DEBUG:
            def _format(t: str, color: str):
                return "\"%s\"" % termcolor.colored(t.replace("\n", "\\n").replace("\r", "\\r"), color)

            text = _format(self.tokenizer.decode([token.item()]), "green")
            text = text.ljust(40)
            # for tok, logprob in sorted(logprobs.items(), key=lambda x: -x[-1]):
            #     text += " %i %s" % (tok, _format(self.tokenizer.decode([tok]), "yellow"))
            #     text += " %0.2f%%" % (100 * math.exp(logprob))
            logging.getLogger("MODEL").info("%6.1fms %s" % (1000 * (time.time() - self.t0), text))
        self.scratchpad.after_token_selection(None, token)
        return bool(self.scratchpad.finish_reason)


class SMCStream(TextStreamer):
    def __init__(self, tokenizer, request_id: str, upload_proxy: UploadProxy,
                 upload_proxy_args: dict, scratchpad: ScratchpadHuggingfaceBase):
        super().__init__(tokenizer)
        self.scratchpad = scratchpad
        self.request_id = request_id
        self.upload_proxy = upload_proxy
        self.upload_proxy_args = upload_proxy_args

    def put(self, value):
        if self.upload_proxy_args.get("ts_first_token", 0) == 0:
            self.upload_proxy_args["ts_first_token"] = time.time()
        super().put(value)

    def on_finalized_text(self, text: str, stream_end: bool = False):
        if self.scratchpad.needs_upload or stream_end:
            if not stream_end:
                self.scratchpad.needs_upload = False
                if self.request_id in self.upload_proxy.check_cancelled():
                    self.scratchpad.finish_reason = "cancelled"
                    return
            self.upload_proxy.upload_result(
                **self.upload_proxy_args,
                files=[self.scratchpad.completion(True)],
                finish_reason=[self.scratchpad.finish_reason],
                generated_tokens_n=[self.scratchpad.generated_tokens_n],
                more_toplevel_fields=[{}],
                status="completed" if stream_end else "in_progress"
            )


class InferenceHF(InferenceBase, LoraLoaderMixin):

    def __init__(self,
                 model_name: str,
                 model_dict: Dict[str, Any],
                 model_cfg: Optional[Dict] = None,
                 load_lora: Optional[str] = None,
                 **kwargs):
        LoraLoaderMixin.__init__(self, load_lora)

        self._model_name = model_name
        self._model_dict = model_dict
        self._model_cfg = model_cfg
        self._model_dir = f"models--{self._model_dict['model_path'].replace('/', '--')}"

        assert torch.cuda.is_available(), "model is only supported on GPU"

        self._device = "cuda:0"
        token = huggingface_hub_token()
        logging.getLogger("MODEL").info("loading model")
        self._tokenizer = AutoTokenizer.from_pretrained(
            self._model_dict["model_path"], cache_dir=self.cache_dir,
            trust_remote_code=True, token=token,
        )
        if model_dict["backend"] == "transformers":
            torch_dtype_mapping = {
                "auto": "auto",
                "fp16": torch.float16,
                "bf16": torch.bfloat16,
            }
            torch_dtype = self._model_dict["model_class_kwargs"].pop("torch_dtype", "auto")
            torch_dtype = torch_dtype_mapping[torch_dtype]
            self._model = AutoModelForCausalLM.from_pretrained(
                self._model_dict["model_path"], cache_dir=self.cache_dir,
                device_map="auto", torch_dtype=torch_dtype, trust_remote_code=True,
                token=token, **self._model_dict["model_class_kwargs"]
            )
        else:
            raise RuntimeError(f"unknown model backend {model_dict['backend']}")
        self._dump_embeddings()

    @property
    def model(self) -> torch.nn.Module:
        return self._model

    @property
    def model_name(self) -> str:
        return self._model_name

    @property
    def model_config(self) -> Dict[str, Any]:
        return self._model_cfg

    @property
    def model_dict(self) -> Dict[str, Any]:
        return self._model_dict

    @property
    def cache_dir(self) -> str:
        return env.DIR_WEIGHTS

    def _dump_embeddings(self):
        if self._model_cfg is None:
            logging.getLogger("MODEL").error(f"Skipping embeddings dumping for the model {self._model_name}")
            return

        for name in chain(self._model_cfg["freeze_exceptions_mapping"]["wte"],
                          self._model_cfg["freeze_exceptions_mapping"]["lm_head"]):
            param = find_param_by_name(model=self._model, name=name)
            torch.save(param, f"{self.cache_dir}/{self._model_dir}/{name}")

    def load_embeddings(self):
        if self._model_cfg is None:
            raise RuntimeError(f"model {self._model_name} has no finetune configuration")

        for name in chain(self._model_cfg["freeze_exceptions_mapping"]["wte"],
                          self._model_cfg["freeze_exceptions_mapping"]["lm_head"]):
            param = find_param_by_name(model=self._model, name=name)
            weights = torch.load(f"{self.cache_dir}/{self._model_dir}/{name}", map_location=self._device)
            param.data.copy_(weights)

    def _prepare_scratchpad(self, request: Dict[str, Any]):
        def logger(*args):
            if not DEBUG:
                return
            s = " ".join([str(a) for a in args])
            logging.getLogger("MODEL").info(s)

        assert request["object"] in ["text_completion_req"]
        scratchpad = ScratchpadHuggingfaceCompletion(tokenizer=self._tokenizer, logger=logger, **request)
        p = scratchpad.prompt()
        logger("prompt %i tokens, max_new_tokens %i" % (len(p), request["max_tokens"]))
        if len(p) == 0:
            raise RuntimeError("empty tokens prompt")

        tokens_prompt = torch.tensor(p, device=self._model.device)
        return scratchpad, tokens_prompt

    def infer(self, request: Dict[str, Any], upload_proxy: UploadProxy, upload_proxy_args: Dict):
        t0 = time.time()
        request_id = request["id"]
        try:
            scratchpad, tokens_prompt = self._prepare_scratchpad(request)
            upload_proxy_args["ts_prompt"] = time.time()
            if request_id in upload_proxy.check_cancelled():
                scratchpad.finish_reason = "cancelled"
                return
            if torch.cuda.get_device_capability() < (8, 0):
                torch.backends.cuda.enable_mem_efficient_sdp(False)
            with torch.inference_mode():
                stopping_criteria = StoppingCriteriaList([
                    CancellationStoppingCriteria(scratchpad, request_id, upload_proxy),
                    FeedScratchoadCriteria(self._tokenizer, t0, scratchpad),
                ])
                streamer = SMCStream(self._tokenizer, request_id, upload_proxy, upload_proxy_args, scratchpad)
                temperature = request.get("temperature", 0.2)
                generation_kwargs = dict(input_ids=tokens_prompt.view(1, *tokens_prompt.shape),
                                         streamer=streamer,
                                         max_new_tokens=request["max_tokens"],
                                         stopping_criteria=stopping_criteria,
                                         do_sample=temperature >= 0.05,
                                         return_dict_in_generate=True,
                                         output_scores=True,
                                         begin_suppress_tokens=[scratchpad.eos_token],
                                         top_p=request.get('top_p', 1.0),
                                         temperature=temperature)

                self._model.generate(**generation_kwargs)
            if not scratchpad.finish_reason:
                scratchpad.finish_reason = "length"
            upload_proxy_args["ts_batch_finished"] = time.time()
            upload_proxy.upload_result(
                **upload_proxy_args,
                files=[scratchpad.completion(True)],
                finish_reason=[scratchpad.finish_reason],
                generated_tokens_n=[scratchpad.generated_tokens_n],
                more_toplevel_fields=[{}],
                status="completed"
            )
        except Exception as e:
            logging.getLogger("MODEL").error(e)
            logging.getLogger("MODEL").error(traceback.format_exc())
