import os
import time
import json
import logging

import torch
import traceback

from typing import Dict, Any

from sentence_transformers import SentenceTransformer

from refact_utils.scripts import env
from refact_utils.huggingface.utils import huggingface_hub_token
from self_hosting_machinery.inference import InferenceBase
from self_hosting_machinery.inference.lora_loader_mixin import LoraLoaderMixin


def log(*args):
    logging.getLogger("MODEL").info(*args)


class InferenceEmbeddings(InferenceBase, LoraLoaderMixin):
    def __init__(
            self,
            model_name: str,
            model_dict: Dict[str, Any],
    ):
        LoraLoaderMixin.__init__(self, None)

        self._model_name = model_name
        self._model_dict = model_dict
        self._model_dir = f"models--{self._model_dict['model_path'].replace('/', '--')}"

        if model_dict.get("cpu"):
            self._device = "cpu"
        else:
            self._device = "cuda:0"

        log("loading model")
        self._model = SentenceTransformer(
            self._model_dict["model_path"],
            device=self._device,
            cache_folder=self.cache_dir,
            use_auth_token=huggingface_hub_token(),
        )

    @property
    def model(self) -> torch.nn.Module:
        return self._model

    @property
    def model_name(self) -> str:
        return self._model_name

    @property
    def model_dict(self) -> Dict[str, Any]:
        return self._model_dict

    @property
    def cache_dir(self) -> str:
        return env.DIR_WEIGHTS

    def infer(self, request: Dict[str, Any], upload_proxy: Any, upload_proxy_args: Dict):
        request_id = request["id"]
        try:
            inputs = request["inputs"]
            B = len(inputs)
            log("embeddings B=%d" % B)
            upload_proxy_args["ts_prompt"] = time.time()
            if request_id in upload_proxy.check_cancelled():
                return
            t0 = time.time()
            files = {
                "results": json.dumps(self._model.encode(inputs).tolist()),
            }
            log("/embeddings %0.3fs" % (time.time() - t0))
            # 8   => 0.141s 0.023s
            # 64  => 0.166s 0.060s
            # 128 => 0.214s 0.120s  *1024 => 1.600s
            upload_proxy_args["ts_batch_finished"] = time.time()
            finish_reason = 'DONE'
            upload_proxy.upload_result(
                **upload_proxy_args,
                files=[files],
                finish_reason=[finish_reason],
                generated_tokens_n=[0],
                more_toplevel_fields=[{}],
                status="completed",
            )

        except Exception as e: # noqa
            log(e)
            log(traceback.format_exc())
