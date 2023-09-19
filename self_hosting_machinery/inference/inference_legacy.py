import sys
import os
import torch
import logging
import time
import traceback

from collections import defaultdict

from refact_scratchpads import ScratchpadBase
from refact_scratchpads import ScratchpadCompletion

from refact_models import CodifyModel
from self_hosting_machinery.inference.lora_loader_mixin import LoraLoaderMixin

from self_hosting_machinery.inference import modload
from self_hosting_machinery.inference import InferenceBase
from refact_scratchpads_no_gpu.stream_results import UploadProxy

from self_hosting_machinery import env

from typing import Optional, Dict, Any, List

log = logging.getLogger("MODEL").info
DEBUG = int(os.environ.get("DEBUG", "0"))


class InferenceLegacy(InferenceBase, LoraLoaderMixin):

    def __init__(self,
                 model_name: str,
                 model_dict: Dict[str, Any],
                 force_cpu: bool = False,
                 load_lora: Optional[str] = None,
                 **kwargs):
        LoraLoaderMixin.__init__(self, load_lora)
        self._model_name = model_name
        self._model_dict = model_dict
        self._device = "cuda" if torch.cuda.is_available() and not force_cpu else "cpu"

        try:
            self._model, self._encoding = self._model_setup(
                self._model_dict, self.cache_dir, self._device
            )
        except Exception as e:
            raise RuntimeError(f"model {model_name} loading failed: {e}")

    @property
    def model(self) -> torch.nn.Module:
        return self._model

    @property
    def model_name(self) -> str:
        return self._model_name

    @property
    def cache_dir(self) -> str:
        return env.DIR_WEIGHTS

    def load_embeddings(self):
        from refact_models.checkpoint_loader import _load_filename

        self._model.wte.weight.data[:] = _load_filename(self.cache_dir, 'emb', self._model_name)
        self._model.lm_head.weight.data[:] = _load_filename(self.cache_dir, 'unemb', self._model_name)

    @staticmethod
    def _model_setup(model_dict: Dict, cache_dir: str, device: str):
        if model_dict["model_class"].endswith("CodifyModel"):
            model = CodifyModel.from_pretrained(
                repo_id=model_dict["model_path"],
                path=cache_dir,
                device=device)
            model.T = model.config.T
        else:
            raise RuntimeError(f"unknown model class {model_dict['model_class']}")
        return model.eval(), model.encoding

    def _prepare_scratchpad(self, request: Dict[str, Any]):
        def logger(*args):
            if not DEBUG:
                return
            s = " ".join([str(a) for a in args])
            log(s)

        object_type = request["object"]
        assert object_type in ["diff_completion_req", "text_completion_req", "chat_completion_req"]

        if object_type == "diff_completion_req":
            Klass = modload(self._model_dict["diff_scratchpad_class"])
            scratchpad = Klass(
                enc=self._encoding,
                logger=logger,
                **request)
        elif object_type == "text_completion_req":
            scratchpad = ScratchpadCompletion(
                enc=self._encoding,
                logger=logger,
                **request)
        else:
            Klass = modload(self._model_dict["chat_scratchpad_class"])
            scratchpad = Klass(
                enc=self._encoding,
                logger=logger,
                **request)

        p = scratchpad.prompt(self._model.T)
        if len(p) == 0:
            raise RuntimeError("empty tokens prompt")

        tokens_prompt = torch.tensor(p, device=self._model.device)
        return scratchpad, tokens_prompt

    def _make_mask(self, seq_len: int, past_key_values_length: int):
        if past_key_values_length == 0:
            mask = torch.ones((seq_len, seq_len + past_key_values_length),
                              dtype=torch.bool, device=self._model.device)
            mask = torch.triu(mask, 1)
        else:
            mask = torch.zeros((seq_len, seq_len + past_key_values_length),
                               dtype=torch.bool, device=self._model.device)
        return mask

    def _before_token_selection(
            self,
            logits: torch.Tensor,
            hidden_state: torch.Tensor,
            scratchpad: ScratchpadBase,
    ) -> Dict[str, Any]:
        output = defaultdict(list)
        for k, v in scratchpad.before_token_selection(
                self._model, b=0, logits=logits, heads=dict(x_bte=hidden_state)).items():
            output[k].append(v)
        return output

    def _select_tokens(
            self,
            logits: torch.Tensor,
            tokens: torch.Tensor,
            chosen_tokens: torch.Tensor,
            scratchpad: ScratchpadBase,
            temperatures: torch.Tensor,
            logits_intrusion: Optional[List[Dict[int, float]]] = None,
            **unused,
    ) -> Dict[str, Any]:
        output = defaultdict(list)
        for k, v in scratchpad.select_tokens(
                logits=logits, tokens=tokens, chosen_tokens=chosen_tokens,
                temperatures=temperatures, logits_intrusion=logits_intrusion).items():
            output[k].append(v)
        return output

    def _after_token_selection(
            self,
            logits: torch.Tensor,
            hidden_state: torch.Tensor,
            chosen_tokens: torch.Tensor,
            scratchpad: ScratchpadBase,
            **unused
    ):
        scratchpad.after_token_selection(
            self._model,
            logits=logits,
            heads=dict(x_bte=hidden_state),
            chosen_token=chosen_tokens[0]
        )

    def _generate_using_scratchpad(self,
                                   sequence: torch.Tensor,
                                   scratchpad: ScratchpadBase,
                                   max_length: int) -> torch.Tensor:
        past_key_values = None
        sequence = sequence.unsqueeze(0)
        output_tokens = torch.empty((1, 1), dtype=torch.int64, device=self._model.device)
        chosen_tokens = torch.empty((1, 1), dtype=torch.int64, device="cpu")
        temperatures = torch.tensor([scratchpad.temp], dtype=torch.float32,
                                    device=self._model.device).view(-1, 1, 1) + 1e-3

        t0 = time.time()
        for token_idx in range(max_length):
            if token_idx == 0:
                seq_len, cache_len = sequence.shape[1], 0
                input_tokens = sequence
            else:
                assert past_key_values is not None
                seq_len, cache_len = 1, past_key_values[0][0].shape[2]
                input_tokens = output_tokens
            # TODO: remove for bigcode models
            attention_mask = self._make_mask(seq_len, cache_len)

            hidden_state, past_key_values = self._model(
                input_tokens,
                attention_mask=attention_mask,
                past_key_values=past_key_values,
                use_cache=True)
            logits = self._model.lm_forward(hidden_state)
            logits = logits[:, [-1], :self._encoding.n_vocab]

            before_kwargs = self._before_token_selection(
                logits=logits,
                hidden_state=hidden_state,
                scratchpad=scratchpad)

            select_kwargs = self._select_tokens(
                logits=logits,
                tokens=output_tokens,
                chosen_tokens=chosen_tokens,
                scratchpad=scratchpad,
                temperatures=temperatures,
                **before_kwargs
            )
            if DEBUG and "top3" in select_kwargs:
                sys.stderr.write("%6.1fms %s" % ((1000 * (time.time() - t0)), select_kwargs["top3"][0]) + "\n")
                sys.stderr.flush()

            sequence = torch.cat([sequence, output_tokens], dim=-1)

            self._after_token_selection(
                logits=logits,
                hidden_state=hidden_state,
                chosen_tokens=chosen_tokens,
                scratchpad=scratchpad,
                **before_kwargs,
                **select_kwargs,
            )

            yield sequence[0]

            if scratchpad.finish_reason:
                break

        if not scratchpad.finish_reason:
            scratchpad.finish_reason = "maxlen"

    def infer(self, request: Dict[str, Any], upload_proxy: UploadProxy, upload_proxy_args: Dict):
        request_id = request["id"]
        try:
            scratchpad, tokens_prompt = self._prepare_scratchpad(request)
            upload_proxy_args["ts_prompt"] = time.time()
            if request_id in upload_proxy.check_cancelled():
                scratchpad.finish_reason = "cancelled"
                return
            with torch.inference_mode():
                for idx, _ in enumerate(self._generate_using_scratchpad(
                        tokens_prompt, scratchpad, max_length=request["max_tokens"])):
                    if idx == 0:
                        upload_proxy_args["ts_first_token"] = time.time()
                    if scratchpad.needs_upload:
                        scratchpad.needs_upload = False
                        if request_id in upload_proxy.check_cancelled():
                            scratchpad.finish_reason = "cancelled"
                            break
                        upload_proxy.upload_result(
                            **upload_proxy_args,
                            files=[scratchpad.completion(True)],
                            finish_reason=[scratchpad.finish_reason],
                            more_toplevel_fields=[scratchpad.toplevel_fields()],
                            generated_tokens_n=[scratchpad.generated_tokens_n],
                            status="in_progress"
                        )
            assert scratchpad.finish_reason
            if DEBUG:
                scratchpad.debuglog("finish_reason", scratchpad.finish_reason)
            upload_proxy_args["ts_batch_finished"] = time.time()
            upload_proxy.upload_result(
                **upload_proxy_args,
                files=[scratchpad.completion(True)],
                finish_reason=[scratchpad.finish_reason],
                more_toplevel_fields=[scratchpad.toplevel_fields()],
                generated_tokens_n=[scratchpad.generated_tokens_n],
                status="completed"
            )
        except Exception as e:
            logging.error(e)
            logging.error(traceback.format_exc())
