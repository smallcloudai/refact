import json
import logging
import os
import struct
from pathlib import Path
from typing import Optional, Dict, Any

import safetensors
import torch
from peft import PeftConfig, get_peft_model
from safetensors.torch import load_file

from refact_utils.finetune.utils import get_active_loras
from refact_utils.scripts import best_lora
from refact_utils.scripts import env
from self_hosting_machinery.finetune.modelling.lora import LoraMixin
from self_hosting_machinery.finetune.modelling.utils import map_model_specific_params

log = logging.getLogger("MODEL").info


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
    def peft_model(self) -> Optional[torch.nn.Module]:
        raise NotImplementedError()

    def set_peft_model(self, model: torch.nn.Module):
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

    def _disable_lora(self):
        if self.peft_model is not None:
            self.peft_model.disable_adapter()
        LoraMixin.exclude_lora(self.model)

    def lora_switch(self, *, lora_checkpoint_dir: str):
        on = not not lora_checkpoint_dir
        if self._lora_on and not on:
            log("deactivating lora")
            self._disable_lora()
            self.load_embeddings()
            self._lora_on = False
        elif not self._lora_on and on:
            log("activating lora %s" % lora_checkpoint_dir)
            self.load_checkpoint(lora_checkpoint_dir, reinstall_lora=True)
            self._lora_checkpoint_dir = lora_checkpoint_dir
            self._lora_on = True
            log("using lora %s" % lora_checkpoint_dir)
        elif self._lora_on and self._lora_checkpoint_dir != lora_checkpoint_dir:
            try:
                self.load_checkpoint(lora_checkpoint_dir, reinstall_lora=False)
            except RuntimeError as e:
                log("failed to quick load lora checkpoint: %s" % e)
                log("will try to remove lora and add again")
                self._disable_lora()
                self._lora_checkpoint_dir = ""
                self._lora_on = False
                self.load_checkpoint(lora_checkpoint_dir, reinstall_lora=True)
            self._lora_checkpoint_dir = lora_checkpoint_dir
            self._lora_on = True
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
        names = set(p.name for p in load_cp_paths)
        if "adapter_model.safetensors" in names:
            self._load_checkpoint(load_path, reinstall_lora)
        else:
            self._legacy_load_checkpoint(load_path, reinstall_lora)

    def _load_checkpoint(
            self,
            load_path: str,
            reinstall_lora: bool = False
    ):
        log("Loading peft format checkpoint")

        LoraMixin.exclude_lora(self.model)

        load_path = Path(load_path)
        tag = f"{load_path.parent.parent.name}_{load_path.name}"
        embeddings_path = load_path / "new_embeddings.safetensors"

        adapter_config = PeftConfig.from_pretrained(load_path)
        adapter_config.inference_mode = True

        if self.peft_model is None:
            self.set_peft_model(get_peft_model(self.model, adapter_config, tag))

        if reinstall_lora:
            self.peft_model.disable_adapter()

        self.peft_model.add_adapter(tag, adapter_config)
        self.peft_model.set_adapter(tag)

        if embeddings_path.exists():
            weights = safetensors.torch.load_file(str(embeddings_path))
            missing, unexpected = self.peft_model.load_state_dict(weights, strict=False)
            if len(unexpected) > 0:
                raise RuntimeError(f"Unexpected keys in finetune checkpoint: {unexpected}")

    def _legacy_load_checkpoint(
            self,
            load_path: str,
            reinstall_lora: bool = False
    ):
        log("Loading legacy format checkpoint")

        if self.peft_model is not None:
            self.peft_model.disable_adapter()

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
