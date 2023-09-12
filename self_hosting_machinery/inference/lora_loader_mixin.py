import os
import torch
import logging

from refact_models.lora import LoraMixin

from self_hosting_machinery.scripts import best_lora
from refact_models.checkpoint_loader import load_finetune_checkpoint
from refact_models.checkpoint_loader import load_finetune_checkpoint_only
from refact_models.checkpoint_loader import load_checkpoint_embeddings
from known_models_db.refact_known_models import models_mini_db

from refact_data_pipeline.finetune.finetune_utils import get_active_loras

from self_hosting_machinery import env

from typing import Optional

log = logging.getLogger("MODEL").info


class LoraLoaderMixin:

    @property
    def model(self) -> torch.nn.Module:
        raise NotImplementedError()

    @property
    def model_name(self) -> str:
        raise NotImplementedError()

    @property
    def cache_dir(self) -> str:
        raise NotImplementedError()

    def __init__(self, load_lora: Optional[str]):
        self._lora_on = False
        self._lora_checkpoint_dir = ""
        if load_lora is not None:
            self.lora_switch(lora_checkpoint_dir=load_lora)

    def lora_switch(self, *, lora_checkpoint_dir: str):
        on = not not lora_checkpoint_dir
        if self._lora_on and not on:
            log("deactivating lora")
            LoraMixin.exclude_lora(self.model)
            # load_checkpoint_embeddings(self.model, self.cache_dir, self.model.model_name)
            self._lora_on = False
        elif not self._lora_on and on:
            log("activating lora %s" % lora_checkpoint_dir)
            load_finetune_checkpoint(self.model, lora_checkpoint_dir)
            self._lora_checkpoint_dir = lora_checkpoint_dir
            self._lora_on = True
        elif self._lora_on and self._lora_checkpoint_dir != lora_checkpoint_dir:
            try:
                load_finetune_checkpoint_only(self.model, lora_checkpoint_dir)
            except RuntimeError as e:
                log("failed to quick load lora checkpoint: %s" % e)
                log("will try to remove lora and add again")
                LoraMixin.exclude_lora(self.model)
                self._lora_checkpoint_dir = ""
                self._lora_on = False
                load_finetune_checkpoint(self.model, lora_checkpoint_dir)
                self._lora_checkpoint_dir = lora_checkpoint_dir
                self._lora_on = True
        if lora_checkpoint_dir:
            log("using lora %s" % lora_checkpoint_dir)

    def lora_switch_according_to_config(self):
        if self.model_name not in models_mini_db:
            raise RuntimeError(f"Unknown model {self.model_name}, try to update repo")
        model_info = models_mini_db[self.model_name]
        if "finetune" not in model_info.get("filter_caps", []):
            log(f"Model {self.model_name} does not support finetune")
            self.lora_switch(lora_checkpoint_dir="")
            return

        active_loras = get_active_loras()
        assert self.model_name in active_loras
        cfg = active_loras[self.model_name]
        # {
        #     "lora_mode": "specific",
        #     "specific_lora_run_id": "lora-20230614-164840",
        #     "specific_checkpoint": "iter0666"
        # }

        if cfg["lora_mode"] not in ["specific", "latest-best"]:
            self.lora_switch(lora_checkpoint_dir="")
            return
        lora_checkpoint_dir = ""
        some_problem_with_explicit = False
        if cfg["lora_mode"] == "specific":
            t = os.path.join(env.DIR_LORAS, cfg["specific_lora_run_id"], "checkpoints", cfg["specific_checkpoint"])
            if os.path.isdir(t):
                lora_checkpoint_dir = t
            else:
                log("lora cannot find \"%s\", switching to latest-best" % t)
                some_problem_with_explicit = True
        if cfg["lora_mode"] == "latest-best" or some_problem_with_explicit:
            tmp = best_lora.find_best_lora(self.model_name)
            lora_checkpoint_dir = tmp["path"]
        self.lora_switch(lora_checkpoint_dir=lora_checkpoint_dir)
