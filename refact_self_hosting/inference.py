import json

import torch
import logging
import requests
import time
import traceback

from collections import defaultdict
from contextlib import contextmanager
from pathlib import Path
from threading import Thread
from threading import Lock

from smallcloud.inference_server import head_and_tail

from code_contrast import ScratchpadBase
from code_contrast import ScratchpadDiff
from code_contrast import ScratchpadCompletion
from code_contrast import ScratchpadBigCode

from refact_self_hosting import known_models

from code_contrast import CodifyModel
from code_contrast import HFModel


from typing import Optional, Dict, Any, Iterable, Tuple, List


__all__ = ["Inference", "LockedError"]


class LockedError(Exception):
    pass


@contextmanager
def non_blocking_lock(lock: Lock):
    if not lock.acquire(blocking=False):
        raise LockedError
    try:
        yield lock
    finally:
        lock.release()


class Inference:

    def __init__(self, workdir: Path, force_cpu: bool, model_name: str = ""):
        self._device = "cuda" if torch.cuda.is_available() and not force_cpu else "cpu"

        self._model_lock = Lock()
        self._model: Optional[torch.nn.Module] = None
        self._encoding = None
        self._loaded_model_name = ""
        self._model_name_arg = model_name
        self._model_dict = dict()
        self._last_error = None

        self._model_setup_thread = Thread(
            target=self._model_setup,
            kwargs={
                'workdir': workdir,
                'model_name': model_name
            })
        self._model_setup_thread.start()

    def _prepare_scratchpad(self, request: Dict[str, Any]):
        created_ts = time.time()

        def logger(*args):
            logging.debug(args)

        object_type = request["object"]
        assert object_type in ["diff_completion_req", "text_completion_req"]

        if object_type == "diff_completion_req":
            DiffScratchpadClass = self._model_dict["diff_scratchpad_class"]
            scratchpad = DiffScratchpadClass(
                enc=self._encoding,
                logger=logger,
                created=created_ts,
                **request)
        else:
            scratchpad = ScratchpadCompletion(
                enc=self._encoding,
                logger=logger,
                created=created_ts,
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

    def _generate_scratchpad(self,
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
            if "top3" in select_kwargs:
                # pass DEBUG=1 environment variable to see this
                print("%6.1fms" % (1000*(time.time() - t0)), select_kwargs["top3"][0])

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

    def _model_setup(self, model_name: str, workdir: Path):
        fetch_timeout = 300
        while True:
            if model_name not in known_models.models_mini_db:
                logging.error(f"unknown model \"{model_name}\", try upgrading this repo")
                time.sleep(fetch_timeout)
                continue

            if model_name == self._loaded_model_name:
                time.sleep(fetch_timeout)
                continue
            self._model_dict = known_models.models_mini_db[model_name]
            with self._model_lock:
                try:
                    self._loaded_model_name = None
                    self._last_error = None
                    if self._model_dict["model_class"] == CodifyModel:
                        self._model = CodifyModel.from_pretrained(
                            str(workdir / "weights"), device=self._device, repo_id=self._model_dict["model_path"])
                        self._model.T = self._model.config.T
                    else:
                        self._model = HFModel.from_pretrained(self._model_dict["model_path"])
                        self._model.T = self._model_dict["T"]
                    self._model = self._model.eval()
                    self._encoding = self._model.encoding
                    self._loaded_model_name = model_name
                    logging.info(f"model {model_name} loaded sucessfully")
                except Exception as e:
                    self._model = None
                    self._encoding = None
                    self._loaded_model_name = None
                    self._last_error = f"model {model_name} loading failed: {e}"
                    logging.error(self._last_error)
            time.sleep(fetch_timeout)

    @staticmethod
    def _json_result(scratchpad: ScratchpadBase, model: str, stream: bool, status: str) -> Optional[Dict]:
        assert status in ["in_progress", "completed"]

        if not scratchpad.needs_upload and status not in ["completed"]:
            return None

        if isinstance(scratchpad, ScratchpadCompletion):
            completion = scratchpad.completion(final=bool(status == "completed"))
        else:
            completion = {"files": scratchpad.completion(final=bool(status == "completed"))}

        result = {
            "id": scratchpad.id,
            "object": "text_completion",
            "status": status,
            "created": scratchpad.created,
            "uploaded": time.time(),
            "generated_tokens_n": scratchpad.generated_tokens_n,
            "model": model,
            "choices": [
                {
                    "index": 0,
                    "logprobs": None,
                    "finish_reason": scratchpad.finish_reason,
                    **completion,
                },
            ],
            **scratchpad.toplevel_fields(),
        }

        if stream and isinstance(scratchpad, ScratchpadDiff):
            for choice in result["choices"]:
                files_head_mid_tail = dict()
                generated = choice.pop("files")
                for filename in generated.keys():
                    orig = scratchpad.sources[filename]
                    dest = generated[filename]
                    if not orig.endswith("\n"):
                        orig += "\n"
                    head, tail = head_and_tail(orig, dest)
                    files_head_mid_tail[filename] = {
                        "head": head,
                        "mid": dest[head:-tail],
                        "tail": tail,
                    }
                choice["files_head_mid_tail"] = files_head_mid_tail

        return result

    def infer(self, request: Dict[str, Any], stream: bool) -> Iterable[Optional[Dict[str, Any]]]:
        try:
            with non_blocking_lock(self._model_lock):
                scratchpad, tokens_prompt = self._prepare_scratchpad(request)
                with torch.inference_mode():
                    for _ in self._generate_scratchpad(tokens_prompt, scratchpad, max_length=request["max_tokens"]):
                        yield self._json_result(
                            scratchpad,
                            model=self._loaded_model_name,
                            stream=stream,
                            status="in_progress")
                assert scratchpad.finish_reason
                yield self._json_result(
                    scratchpad,
                    model=self._loaded_model_name,
                    stream=stream,
                    status="completed")
        except Exception as e:
            logging.error(e)
            logging.error(traceback.format_exc())
            yield None

    @property
    def model_name(self):
        return self._loaded_model_name

    @property
    def last_error(self):
        return self._last_error

    @property
    def longthink_functions(self) -> Dict:
        if 'longthink_functions' in self._model_dict:
            return self._model_dict['longthink_functions']
        return {}
