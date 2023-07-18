import sys
import os
import torch
import logging
import time
import traceback
import json

from collections import defaultdict

from refact_scratchpads import ScratchpadBase
from refact_scratchpads import ScratchpadCompletion

from refact_models import CodifyModel
from refact_models import RefactModel
from refact_models import HFModel
from refact_models import StarChatModel

from self_hosting_machinery.scripts import best_lora
from refact_models.checkpoint_loader import load_finetune_checkpoint
from refact_models.checkpoint_loader import load_finetune_checkpoint_only
from refact_models.checkpoint_loader import load_checkpoint_embeddings

from self_hosting_machinery.inference import modload
from self_hosting_machinery.inference import InferenceBase
from refact_scratchpads_no_gpu.stream_results import UploadProxy

from self_hosting_machinery import env

from typing import Optional, Dict, Any, List


log = logging.getLogger("MODEL").info
DEBUG = int(os.environ.get("DEBUG", "0"))


class InferenceLegacy(InferenceBase):

    def __init__(self,
                 model_name: str,
                 model_dict: Dict[str, Any],
                 force_cpu: bool = False,
                 load_lora: Optional[str] = None):
        self._model_name = model_name
        self._model_dict = model_dict
        self._device = "cuda" if torch.cuda.is_available() and not force_cpu else "cpu"

        try:
            self._model, self._encoding = self._model_setup(
                self._model_dict, self._device)
        except Exception as e:
            raise RuntimeError(f"model {model_name} loading failed: {e}")

        self._lora_on = False
        self._lora_checkpoint_dir = ""
        if load_lora is not None:
            self.lora_switch(on=True, lora_checkpoint_dir=load_lora)

    @staticmethod
    def _model_setup(model_dict: Dict, device: str):
        if model_dict["model_class"].endswith("CodifyModel"):
            model = CodifyModel.from_pretrained(
                repo_id=model_dict["model_path"],
                path=env.DIR_WEIGHTS,
                device=device)
            model.T = model.config.T
        elif model_dict["model_class"].endswith("RefactModel"):
            model = RefactModel.from_pretrained(
                repo_id=None,
                path=model_dict["model_path"],
                device=device)
            model.T = model.config.T
        elif model_dict["model_class"].endswith("HFModel"):
            model = HFModel.from_pretrained(
                path=model_dict["model_path"],
                cache_dir=env.DIR_WEIGHTS,
                device=device)
            model.T = model_dict["T"]
        elif model_dict["model_class"].endswith("StarChatModel"):
            model = StarChatModel.from_pretrained(
                path=model_dict["model_path"],
                cache_dir=env.DIR_WEIGHTS,
                device=device)
            model.T = model_dict["T"]
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
                sys.stderr.write("%6.1fms %s" % ((1000*(time.time() - t0)), select_kwargs["top3"][0]) + "\n")
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

    def lora_switch(self, *, lora_checkpoint_dir: str):
        on = not not lora_checkpoint_dir
        if self._lora_on and not on:
            log("deactivating lora")
            self._model = self._model.exclude_lora(self._model)
            self._model = load_checkpoint_embeddings(self._model, self._model.cache_dir, self._model.model_name)
            self._lora_on = False
        elif not self._lora_on and on:
            log("activating lora %s" % lora_checkpoint_dir)
            self._model = load_finetune_checkpoint(self._model, lora_checkpoint_dir)
            self._lora_checkpoint_dir = lora_checkpoint_dir
            self._lora_on = True
        elif self._lora_on and self._lora_checkpoint_dir != lora_checkpoint_dir:
            try:
                self._model = load_finetune_checkpoint_only(self._model, lora_checkpoint_dir)
            except RuntimeError as e:
                log("failed to quick load lora checkpoint: %s" % e)
                log("will try to remove lora and add again")
                self._model = self._model.exclude_lora(self._model)
                self._lora_checkpoint_dir = ""
                self._lora_on = False
                self._model = load_finetune_checkpoint(self._model, lora_checkpoint_dir)
                self._lora_checkpoint_dir = lora_checkpoint_dir
                self._lora_on = True
        if lora_checkpoint_dir:
            log("using lora %s" % lora_checkpoint_dir)

    def lora_switch_according_to_config(self):
        if not os.path.exists(env.CONFIG_ACTIVE_LORA):
            j = {
                "model": "",
                "lora_mode": "latest-best",
            }
        else:
            j = json.load(open(env.CONFIG_ACTIVE_LORA))
        # {
        #     "model": "",
        #     "lora_mode": "specific",
        #     "specific_lora_run_id": "lora-20230614-164840",
        #     "specific_checkpoint": "iter0666"
        # }
        # NOTE: lora only for 3b model now
        if self._model_name not in ["CONTRASTcode/3b/multi"]:
            log("lora disabled for %s" % self._model_name)
            self.lora_switch(lora_checkpoint_dir="")
            return
        if j["lora_mode"] not in ["specific", "latest-best"]:
            self.lora_switch(lora_checkpoint_dir="")
            return
        lora_checkpoint_dir = ""
        some_problem_with_explicit = False
        if j["lora_mode"] == "specific":
            t = os.path.join(env.DIR_LORAS, j["specific_lora_run_id"], "checkpoints", j["specific_checkpoint"])
            if os.path.isdir(t):
                lora_checkpoint_dir = t
            else:
                log("lora cannot find \"%s\", switching to latest-best" % t)
                some_problem_with_explicit = True
        if j["lora_mode"] == "latest-best" or some_problem_with_explicit:
            tmp = best_lora.find_best_lora(self._model_name)
            lora_checkpoint_dir = tmp["path"]
        self.lora_switch(lora_checkpoint_dir=lora_checkpoint_dir)
