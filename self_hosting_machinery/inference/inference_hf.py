import logging
import os
import time
import torch
import traceback

from auto_gptq import AutoGPTQForCausalLM
from transformers import AutoModelForCausalLM
from transformers import AutoTokenizer
from transformers import StoppingCriteria
from transformers import StoppingCriteriaList
from transformers.generation.streamers import TextStreamer

from refact_scratchpads import ScratchpadHuggingfaceBase
from refact_scratchpads import ScratchpadHuggingfaceCompletion
from self_hosting_machinery.inference import InferenceBase
from self_hosting_machinery.inference import modload
from refact_scratchpads_no_gpu.stream_results import UploadProxy

from self_hosting_machinery import env

from typing import Dict, Any, Union, Optional


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


class StopTokenStoppingCriteria(StoppingCriteria):

    def __init__(self, scratchpad: ScratchpadHuggingfaceBase):
        StoppingCriteria.__init__(self)
        self.scratchpad = scratchpad

    def __call__(self, input_ids: torch.LongTensor, scores: torch.FloatTensor, **kwargs) -> bool:
        last_tokens = input_ids[0][-1]
        self.scratchpad.after_token_selection(None, last_tokens)
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


# NOTE: original class AutoGPTQForCausalLM do not handle cache_dir, so we customized it
class CustomAutoGPTQForCausalLM(AutoGPTQForCausalLM):

    @classmethod
    def from_quantized(
            cls,
            model_name_or_path: str,
            device: Optional[Union[str, int]] = None,
            model_basename: Optional[str] = None,
            use_safetensors: bool = True,
            trust_remote_code: bool = True,
            use_triton=False,
            quantize_config=None,
            **kwargs):
        from auto_gptq.modeling.auto import check_and_get_model_type
        from auto_gptq.modeling.auto import GPTQ_CAUSAL_LM_MODEL_MAP
        model_type = check_and_get_model_type(model_name_or_path, trust_remote_code)
        quant_func = GPTQ_CAUSAL_LM_MODEL_MAP[model_type].from_quantized
        return quant_func(
            model_name_or_path=model_name_or_path,
            device=device,
            model_basename=model_basename,
            use_safetensors=use_safetensors,
            trust_remote_code=trust_remote_code,
            use_triton=use_triton,
            quantize_config=quantize_config,
            **kwargs)


class InferenceHF(InferenceBase):

    def __init__(self,
                 model_name: str,
                 model_dict: Dict[str, Any],
                 **kwargs):
        self._model_name = model_name
        self._model_dict = model_dict

        assert torch.cuda.is_available(), "model is only supported on GPU"

        self._device = "cuda:0"
        self._tokenizer = AutoTokenizer.from_pretrained(
            self._model_dict["model_path"], cache_dir=env.DIR_WEIGHTS, trust_remote_code=True)

        if model_dict["backend"] == "transformers":
            self._model = AutoModelForCausalLM.from_pretrained(
                self._model_dict["model_path"], cache_dir=env.DIR_WEIGHTS, device_map="auto",
                trust_remote_code=True, **self._model_dict["model_class_kwargs"])
        elif model_dict["backend"] == "autogptq":
            self._model = CustomAutoGPTQForCausalLM.from_quantized(
                self._model_dict["model_path"], cache_dir=env.DIR_WEIGHTS, device=self._device,
                trust_remote_code=True, **self._model_dict["model_class_kwargs"])
        else:
            raise RuntimeError(f"unknown model backend {model_dict['backend']}")

    def _prepare_scratchpad(self, request: Dict[str, Any]):
        def logger(*args):
            if not DEBUG:
                return
            s = " ".join([str(a) for a in args])
            logging.getLogger("MODEL").info(s)

        object_type = request["object"]
        assert object_type in ["diff_completion_req", "text_completion_req", "chat_completion_req"]
        if object_type == "diff_completion_req":
            Scratchpad = modload(self._model_dict["diff_scratchpad_class"])
        elif object_type == "chat_completion_req":
            Scratchpad = modload(self._model_dict["chat_scratchpad_class"])
        else:
            Scratchpad = ScratchpadHuggingfaceCompletion

        scratchpad = Scratchpad(tokenizer=self._tokenizer, logger=logger, **request)
        p = scratchpad.prompt(self._tokenizer.max_len_single_sentence)
        if len(p) == 0:
            raise RuntimeError("empty tokens prompt")

        tokens_prompt = torch.tensor(p, device=self._model.device)
        return scratchpad, tokens_prompt

    def infer(self, request: Dict[str, Any], upload_proxy: UploadProxy, upload_proxy_args: Dict):
        request_id = request["id"]
        try:
            scratchpad, tokens_prompt = self._prepare_scratchpad(request)
            upload_proxy_args["ts_prompt"] = time.time()
            if request_id in upload_proxy.check_cancelled():
                scratchpad.finish_reason = "cancelled"
                return
            with torch.inference_mode():
                stopping_criteria = StoppingCriteriaList([
                    CancellationStoppingCriteria(scratchpad, request_id, upload_proxy),
                    StopTokenStoppingCriteria(scratchpad),
                ])
                streamer = SMCStream(self._tokenizer, request_id, upload_proxy, upload_proxy_args, scratchpad)
                generation_kwargs = dict(input_ids=tokens_prompt.view(1, *tokens_prompt.shape),
                                         streamer=streamer,
                                         max_new_tokens=request["max_tokens"],
                                         stopping_criteria=stopping_criteria,
                                         return_dict_in_generate=True,
                                         output_scores=True)
                self._model.generate(**generation_kwargs)
            if not scratchpad.finish_reason:
                scratchpad.finish_reason = "maxlen"
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

    def lora_switch_according_to_config(self):
        pass
