import json
import logging
import struct
from pathlib import Path
from typing import Optional, Dict, Any

import safetensors
import torch
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
            log("using lora %s" % lora_checkpoint_dir)
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
            log("using lora %s" % lora_checkpoint_dir)

    def lora_switch_according_to_request(self, lora_config: Optional[Dict[str, str]]):
        lora_checkpoint_dir = ""
        if lora_config is not None:
            lora_checkpoint_dir = str(
                Path(env.DIR_LORAS) / lora_config.get("run_id") / "checkpoints" / lora_config.get("checkpoint_id"))
        self.lora_switch(lora_checkpoint_dir=lora_checkpoint_dir)

    def load_checkpoint(
            self,
            load_path: str,
            reinstall_lora: bool = False
    ):
        load_path = Path(load_path)
        load_cp_paths = [p for p in load_path.iterdir() if p.suffix in {".pt", ".pth", ".safetensors"}]
        if len(load_cp_paths) == 0:
            raise FileNotFoundError(f"No checkpoint found in {load_path}")

        is_new_format = "adapter_config.json" in set(p.name for p in load_path.iterdir())
        old_format_finetune_cp = None
        if is_new_format:
            adapter_config = json.load(open(load_path / "adapter_config.json", 'r'))
            lora_cfg = {
                "lora_target_modules": adapter_config["target_modules"],
                "lora_r": adapter_config["r"],
                "lora_alpha": adapter_config["lora_alpha"],
                "lora_dropout": adapter_config["lora_dropout"]
            }
        else:
            old_format_finetune_cp = _load_filename(load_path / "mp_rank_00_model_states.pt")
            lora_cfg = old_format_finetune_cp['ds_config']['model_info']['lora']

        if reinstall_lora:
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

        if is_new_format:
            embeddings_path = load_path / "new_embeddings.safetensors"
            finetune_weights = safetensors.torch.load_file(Path(load_path) / "adapter_model.safetensors")
            finetune_weights = {k.replace("base_model.model.", ""): v for k, v in finetune_weights.items()}
            missing, unexpected = self.model.load_state_dict(finetune_weights, strict=False)
            if len(unexpected) > 0:
                raise RuntimeError(f"Unexpected keys in finetune checkpoint: {unexpected}")

            if embeddings_path.exists():
                weights = safetensors.torch.load_file(str(embeddings_path))
                weights = {k.replace("base_model.model.", ""): v for k, v in weights.items()}
                missing, unexpected = self.model.load_state_dict(weights, strict=False)
                if len(unexpected) > 0:
                    raise RuntimeError(f"Unexpected keys in finetune checkpoint: {unexpected}")
        else:
            assert old_format_finetune_cp is not None
            missing, unexpected = self.model.load_state_dict(old_format_finetune_cp['module'], strict=False)
            if len(unexpected) > 0:
                raise RuntimeError(f"Unexpected keys in finetune checkpoint: {unexpected}")
