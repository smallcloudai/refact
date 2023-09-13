import os
import logging
from pathlib import Path

from refact_models.config import Config

from typing import Optional

from refact_models.lora import LoraMixin


def _load_gs_file(root_path: str, filename: str):
    import blobfile as bf
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


def load_config(root_path: str, repo_id: Optional[str] = None):
    config = _load_filename(root_path, 'model-hps.json', repo_id)
    return Config.from_dict(config)


def load_checkpoint(model, root_path: str, repo_id: Optional[str] = None):
    model.wte.weight.data[:] = _load_filename(root_path, 'emb', repo_id)
    model.lm_head.weight.data[:] = _load_filename(root_path, 'unemb', repo_id)
    model.ln_f.weight.data[:] = _load_filename(root_path, 'bounce.ln_final.weight', repo_id)
    model.ln_f.bias.data[:] = _load_filename(root_path, 'bounce.ln_final.bias', repo_id)

    model.bidir_sa_ln.weight.data[:] = _load_filename(root_path, 'bidir_sa_ln.weight', repo_id)
    model.bidir_sa_ln.bias.data[:] = _load_filename(root_path, 'bidir_sa_ln.bias', repo_id)
    model.bidir_sa.qkv.weight.data[:] = _load_filename(root_path, 'bidir_sa.qkv', repo_id)
    model.bidir_sa.qkv.bias.data[:] = _load_filename(root_path, 'bidir_sa.qkv_bias', repo_id)
    model.bidir_sa.out.weight.data[:] = _load_filename(root_path, 'bidir_sa.backproj', repo_id)
    model.bidir_sa.out.bias.data[:] = _load_filename(root_path, 'bidir_sa.backproj_bias', repo_id)

    model.bidir_2logits_ln.weight.data[:] = _load_filename(root_path, 'bidir_2logits_ln.weight', repo_id)
    model.bidir_2logits_ln.bias.data[:] = _load_filename(root_path, 'bidir_2logits_ln.bias', repo_id)
    model.bidir_2logits.weight.data[:] = _load_filename(root_path, 'bidir_2logits.weight', repo_id)
    model.bidir_2logits.bias.data[:] = _load_filename(root_path, 'bidir_2logits.bias', repo_id)

    for i in range(1, len(model.blocks) + 1):
        f_prefix = f'layers.{i:03d}'
        model.blocks[i - 1].ln_a.weight.data[:] = _load_filename(root_path, f'{f_prefix}.ln_a.weight', repo_id)
        model.blocks[i - 1].ln_a.bias.data[:] = _load_filename(root_path, f'{f_prefix}.ln_a.bias', repo_id)
        model.blocks[i - 1].ln_m.weight.data[:] = _load_filename(root_path, f'{f_prefix}.ln_m.weight', repo_id)
        model.blocks[i - 1].ln_m.bias.data[:] = _load_filename(root_path, f'{f_prefix}.ln_m.bias', repo_id)

        model.blocks[i - 1].mlp.ln_1.weight.data[:] = _load_filename(root_path, f'{f_prefix}.pw.W1', repo_id)
        model.blocks[i - 1].mlp.ln_1.bias.data[:] = _load_filename(root_path, f'{f_prefix}.pw.b1', repo_id)
        model.blocks[i - 1].mlp.ln_2.weight.data[:] = _load_filename(root_path, f'{f_prefix}.pw.W2', repo_id)
        model.blocks[i - 1].mlp.ln_2.bias.data[:] = _load_filename(root_path, f'{f_prefix}.pw.b2', repo_id)

        model.blocks[i - 1].sa.qkv.weight.data[:] = _load_filename(root_path, f'{f_prefix}.sa.qkv', repo_id)
        model.blocks[i - 1].sa.qkv.bias.data[:] = _load_filename(root_path, f'{f_prefix}.sa.qkv_bias', repo_id)
        model.blocks[i - 1].sa.out.weight.data[:] = _load_filename(root_path, f'{f_prefix}.sa.backproj', repo_id)
        model.blocks[i - 1].sa.out.bias.data[:] = _load_filename(root_path, f'{f_prefix}.sa.backproj_bias', repo_id)


def load_finetune_checkpoint(model, model_name: str, root_path: str, repo_id: Optional[str] = None):
    from refact_data_pipeline.finetune.model_handling import setup_model_specific_params

    finetune_cp = _load_filename(root_path, 'mp_rank_00_model_states.pt', repo_id)
    lora_cfg = finetune_cp['ds_config']['model_info']['lora']
    _, lora_target_modules = setup_model_specific_params(
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

