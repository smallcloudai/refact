import sys
import os
import torch
import logging
import time
import traceback
import signal
import socket
import datetime

from collections import defaultdict

from code_contrast import ScratchpadBase
from code_contrast import ScratchpadCompletion

from refact_self_hosting import known_models

from code_contrast.modeling import CodifyModel
from code_contrast.modeling import HFModel
from code_contrast.modeling import GPTQBigCodeModel
from code_contrast.modeling.checkpoint_loader import load_finetune_checkpoint
from typing import Optional, Dict, Any, List

from refact_self_hosting import env

from smallcloud import inference_server
inference_server.override_urls("http://127.0.0.1:8008/infengine-v1/")


log = logging.getLogger("MODEL").info


quit_flag = False
DEBUG = int(os.environ.get("DEBUG", "0"))
if DEBUG:
    inference_server.DEBUG_UPLOAD_NOT_SEPARATE_PROCESS = True


class Inference:

    def __init__(self,
                 model_name: str,
                 force_cpu: bool = False,
                 load_lora: Optional[str] = None):
        if model_name not in known_models.models_mini_db:
            raise RuntimeError(f"unknown model \"{model_name}\", try upgrading this repo")
        self._model_name = model_name
        self._model_dict = known_models.models_mini_db[model_name]
        self._device = "cuda" if torch.cuda.is_available() and not force_cpu else "cpu"

        try:
            self._model, self._encoding = self._model_setup(
                self._model_dict, self._device)
            if load_lora is not None:
                self._model = load_finetune_checkpoint(self._model, load_lora)
        except Exception as e:
            raise RuntimeError(f"model {model_name} loading failed: {e}")

    @staticmethod
    def _model_setup(model_dict: Dict, device: str):
        if model_dict["model_class"] == CodifyModel:
            model = CodifyModel.from_pretrained(
                repo_id=model_dict["model_path"],
                path=env.DIR_WEIGHTS,
                device=device)
            model.T = model.config.T
        elif model_dict["model_class"] == HFModel:
            model = HFModel.from_pretrained(
                path=model_dict["model_path"],
                cache_dir=env.DIR_WEIGHTS,
                device=device)
            model.T = model_dict["T"]
        elif model_dict["model_class"] == GPTQBigCodeModel:
            model = GPTQBigCodeModel(
                model_name=model_dict["model_path"],
                cache_dir=env.DIR_WEIGHTS,
                device=device,
                **model_dict["model_class_kwargs"])
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
            DiffScratchpadClass = self._model_dict["diff_scratchpad_class"]
            scratchpad = DiffScratchpadClass(
                enc=self._encoding,
                logger=logger,
                **request)
        elif object_type == "text_completion_req":
            scratchpad = ScratchpadCompletion(
                enc=self._encoding,
                logger=logger,
                **request)
        else:
            ChatScratchpadClass = self._model_dict["chat_scratchpad_class"]
            scratchpad = ChatScratchpadClass(
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

    def infer(self, request: Dict[str, Any], upload_proxy: inference_server.UploadProxy, upload_proxy_args: Dict):
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


def worker_loop(model_name: str, cpu: bool, load_lora: str, compile: bool):
    stream_handler = logging.StreamHandler(stream=sys.stdout)
    logging.basicConfig(level=logging.INFO, handlers=[stream_handler])

    inference_model = Inference(model_name=model_name, force_cpu=cpu, load_lora=load_lora)
    class DummyUploadProxy:
        def upload_result(*args, **kwargs):
            pass
        def check_cancelled():
            return set()
    dummy_calls = [
        {
            'function': 'completion',
            'temperature': 0.8, 'top_p': 0.95, 'max_tokens': 40, 'id': 'comp-wkCX57Le8giP-1337', 'created': 0,
            'stop_tokens': [],
            'prompt': 'Hello world',
            'echo': False,
            'object': 'text_completion_req',
        }
    ]
    inference_model.infer(dummy_calls[0], DummyUploadProxy, {})
    if compile:
        return

    req_session = inference_server.infserver_session()
    description_dict = inference_server.validate_description_dict(
        model_name + "_" + socket.getfqdn(),
        "account_name",
        model=model_name, B=1, max_thinking_time=10,
    )
    upload_proxy = inference_server.UploadProxy(
        upload_q=None, cancelled_q=None)
    upload_proxy.start_upload_result_daemon()

    while not quit_flag:
        upload_proxy.keepalive()
        upload_proxy.cancelled_reset()
        retcode, request_batch = inference_server.completions_wait_batch(
            req_session, description_dict, verbose=False)
        ts_arrived = time.time()
        if retcode == "OK":
            for request in request_batch:
                upload_proxy_args = {
                    "description_dict": description_dict,
                    "original_batch": [request],
                    "idx_updated": [0],
                    "tokens": None,
                    "ts_arrived": ts_arrived,
                    "ts_batch_started": time.time(),
                    "ts_prompt": 0,
                    "ts_first_token": 0,
                    "ts_batch_finished": 0,
                }
                inference_model.infer(request, upload_proxy, upload_proxy_args)
        elif retcode == "WAIT":
            # Normal, no requests
            pass
        else:
            # No connectivity, connection refused, other errors go there
            time.sleep(10)

    upload_proxy.stop()


def catch_sigkill(signum, frame):
    sys.stderr.write("caught SIGUSR1")
    sys.stderr.flush()
    global quit_flag
    quit_flag = True


if __name__ == "__main__":
    from argparse import ArgumentParser

    parser = ArgumentParser()
    parser.add_argument("--model", type=str)
    parser.add_argument("--cpu", action="store_true")
    parser.add_argument("--load-lora")
    parser.add_argument("--compile", action="store_true", help="download and compile triton kernels, quit")
    args = parser.parse_args()

    class MyLogHandler(logging.Handler):
        def emit(self, record):
            timestamp = datetime.datetime.now().strftime("%Y%m%d %H:%M:%S")
            sys.stderr.write(timestamp + " " + self.format(record) + "\n")
            sys.stderr.flush()
    handler = MyLogHandler()
    handler.setLevel(logging.INFO)
    handler.setFormatter(logging.Formatter('MODEL %(message)s'))
    root = logging.getLogger()
    root.addHandler(handler)
    root.setLevel(logging.INFO)

    signal.signal(signal.SIGUSR1, catch_sigkill)

    worker_loop(args.model, args.cpu, args.load_lora, compile=args.compile)
