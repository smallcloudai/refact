import logging
import os
from pathlib import Path
from typing import Optional

import blobfile as bf

from self_hosting_machinery.finetune.modelling.lora import LoraMixin


def _load_gs_file(root_path: str, filename: str):
    rest = root_path[len("gs://"):]
    slash = '/'
    if root_path[-1] == '/':
        slash = ''
    local = os.path.join("/tmp/small-cache-container", rest, filename)
    os.makedirs(os.path.dirname(local), exist_ok=True)
    path = f'{root_path}{slash}{filename}'
    if os.path.exists(local):
        logging.info("using cached %s" % local)
    else:
        logging.info("download %s" % (path))
        bf.copy(path, local + ".tmp")
        os.rename(local + ".tmp", local)
    return str(local)


def _load_filename(root_path: str, filename: str, repo_id: Optional[str] = None):
    if repo_id is None:
        if root_path.startswith('gs://'):
            local_path = _load_gs_file(root_path, filename)
            local_path = Path(local_path)
        else:
            local_path = Path(root_path) / filename
    else:
        from huggingface_hub import hf_hub_download
        args = dict(
            repo_id=repo_id,
            filename=filename,
            cache_dir=root_path,
        )
        try:
            local_path = hf_hub_download(**args, local_files_only=True)
        except FileNotFoundError:
            while True:
                try:
                    local_path = hf_hub_download(**args, local_files_only=False)
                    break
                except Exception as e:
                    print('retrying...')
                    continue
            print("saved \"%s\"" % local_path)
        local_path = Path(local_path)

    if not local_path.exists():
        raise RuntimeError(f"Not found: {local_path}")

    # logging.info(f'load {local_path}')
    if local_path.suffix == ".json":
        import json
        return json.loads(local_path.read_text())
    elif local_path.suffix in {'.pt', '.pth'}:
        import torch
        return torch.load(local_path, map_location='cpu')
    else:
        import cloudpickle
        return cloudpickle.loads(local_path.read_bytes())


def load_finetune_checkpoint(model, model_name: str, root_path: str, repo_id: Optional[str] = None):
    from self_hosting_machinery.finetune.modelling.utils import map_model_specific_params

    finetune_cp = _load_filename(root_path, 'mp_rank_00_model_states.pt', repo_id)
    lora_cfg = finetune_cp['ds_config']['model_info']['lora']
    _, lora_target_modules = map_model_specific_params(
        model_name, lora_target_modules=lora_cfg.pop('lora_target_modules'), freeze_exceptions=[]
    )
    LoraMixin.apply_lora(
        model,
        lora_target_modules=lora_target_modules,
        **lora_cfg
    )
    missing, unexpected = model.load_state_dict(finetune_cp['module'], strict=False)
    if len(unexpected) > 0:
        raise RuntimeError(f"Unexpected keys in finetune checkpoint: {unexpected}")


def load_finetune_checkpoint_only(model, root_path: str):
    finetune_cp = _load_filename(root_path, 'mp_rank_00_model_states.pt', None)
    missing, unexpected = model.load_state_dict(finetune_cp['module'], strict=False)
    if len(unexpected) > 0:
        raise RuntimeError(f"Unexpected keys in finetune checkpoint: {unexpected}")
