import os
import time
import json

import torch
import traceback

from typing import Dict, Any

from self_hosting_machinery import env
from sentence_transformers import SentenceTransformer
from self_hosting_machinery.inference import log
from self_hosting_machinery.inference import logger
from self_hosting_machinery.inference import InferenceBase
from self_hosting_machinery.inference.lora_loader_mixin import LoraLoaderMixin


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

        assert torch.cuda.is_available(), "model is only supported on GPU"

        self._device = "cuda:0"
        for local_files_only in [True, False]:
            try:
                # WARNING: this may not work if you have no access to the web as it may try to download tokenizer
                log("loading model local_files_only=%i" % local_files_only)
                if local_files_only:
                    self._model = SentenceTransformer(
                        os.path.join(self.cache_dir, self._model_dir),
                        device=self._device,
                        cache_folder=self.cache_dir,
                    )
                    break
                else:
                    self._model = SentenceTransformer(
                        self._model_dict["model_path"],
                        device=self._device,
                        cache_folder=self.cache_dir,
                    )
                    self._model.save(os.path.join(self.cache_dir, self._model_dir))
            except Exception as e: # noqa
                if not local_files_only:
                    raise

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

    def infer(self, request: Dict[str, Any], upload_proxy: Any, upload_proxy_args: Dict, log=print):

        request_id = request["id"]
        try:
            upload_proxy_args["ts_prompt"] = time.time()
            if request_id in upload_proxy.check_cancelled():
                return

            files = {
                "results": json.dumps(self._model.encode(request["inputs"]).tolist()),
            }

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
            logger.error(e)
            logger.error(traceback.format_exc())
