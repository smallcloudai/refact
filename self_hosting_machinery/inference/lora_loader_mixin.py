import json
import os
import struct
from pathlib import Path
from typing import Optional, Dict, Any

import torch
from safetensors.torch import load_file

from self_hosting_machinery import env
from self_hosting_machinery.inference import log
from self_hosting_machinery.finetune.modelling.lora import LoraMixin
from self_hosting_machinery.finetune.modelling.utils import map_model_specific_params
from self_hosting_machinery.finetune.utils.finetune_utils import get_active_loras
from self_hosting_machinery.scripts import best_lora


def _load_filename(
        path: Path,
):
    def _parse_safetensors_metadata(path: Path):
        with open(path, 'rb') as f:
            length_of_header = struct.unpack('<Q', f.read(8))[0]
            return json.loads(f.read(length_of_header).decode('utf-8'))

    if not path.exists():
        raise RuntimeError(f"Not found: {path}")

    if path.suffix in {'.pt', '.pth'}:
        return torch.load(path, map_location='cpu')
    elif path.suffix == ".safetensors":
        meta = _parse_safetensors_metadata(path)
        return {
            "module": load_file(path, device='cpu'),
            "ds_config": json.loads(meta["__metadata__"]["ds_config"])
        }
    else:
        raise RuntimeError(f"Unknown file format: {path}")


class LoraLoaderMixin:

    @property
    def model(self) -> torch.nn.Module:
        raise NotImplementedError()

    @property
    def model_name(self) -> str:
        raise NotImplementedError()

    @property
    def model_dict(self) -> Dict[str, Any]:
        raise NotImplementedError()

    @property
    def cache_dir(self) -> str:
        raise NotImplementedError()

    def load_embeddings(self):
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
            self.load_embeddings()
            self._lora_on = False
        elif not self._lora_on and on:
            log("activating lora %s" % lora_checkpoint_dir)
            self.load_checkpoint(lora_checkpoint_dir, reinstall_lora=True)
            self._lora_checkpoint_dir = lora_checkpoint_dir
            self._lora_on = True
        elif self._lora_on and self._lora_checkpoint_dir != lora_checkpoint_dir:
            try:
                self.load_checkpoint(lora_checkpoint_dir, reinstall_lora=False)
            except RuntimeError as e:
                log("failed to quick load lora checkpoint: %s" % e)
                log("will try to remove lora and add again")
                LoraMixin.exclude_lora(self.model)
                self._lora_checkpoint_dir = ""
                self._lora_on = False
                self.load_checkpoint(lora_checkpoint_dir, reinstall_lora=True)
                self._lora_checkpoint_dir = lora_checkpoint_dir
                self._lora_on = True
        if lora_checkpoint_dir:
            log("using lora %s" % lora_checkpoint_dir)

    def lora_switch_according_to_config(self):
        if "finetune" not in self.model_dict.get("filter_caps", []):
            log(f"Model {self.model_name} does not support finetune")
            self.lora_switch(lora_checkpoint_dir="")
            return

        cfg = get_active_loras({
            self.model_name: self.model_dict
        })[self.model_name]
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

    def load_checkpoint(
            self,
            load_path: str,
            reinstall_lora: bool = False
    ):
        load_cp_paths = [p for p in Path(load_path).iterdir() if p.suffix in {".pt", ".pth", ".safetensors"}]
        if len(load_cp_paths) == 0:
            raise FileNotFoundError(f"No checkpoint found in {load_path}")

        finetune_cps = [_load_filename(p) for p in load_cp_paths]
        if len(finetune_cps) > 1:
            raise NotImplementedError("Loading of sharded checkpoint is not implemented")
        finetune_cp = finetune_cps[0]

        if reinstall_lora:
            lora_cfg = finetune_cp['ds_config']['model_info']['lora']
            freeze_exceptions, lora_target_modules = map_model_specific_params(
                model_name=self.model_name,
                freeze_exceptions=[],
                lora_target_modules=lora_cfg.pop('lora_target_modules')
            )
            LoraMixin.apply_lora(
                self.model,
                lora_target_modules=lora_target_modules,
                **lora_cfg
            )

        missing, unexpected = self.model.load_state_dict(finetune_cp['module'], strict=False)
        if len(unexpected) > 0:
            raise RuntimeError(f"Unexpected keys in finetune checkpoint: {unexpected}")
