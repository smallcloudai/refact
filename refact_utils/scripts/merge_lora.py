import os
import json
import tempfile
import subprocess
import safetensors.torch

from transformers import AutoTokenizer
from transformers import AutoModelForCausalLM

from refact_utils.scripts import env
from refact_utils.finetune.utils import is_checkpoint_deprecated
from self_hosting_machinery.finetune.modelling.lora import LoraMixin

from pathlib import Path


class LoraMerger:

    def __init__(self, model_path: str):
        self._tokenizer = AutoTokenizer.from_pretrained(
            model_path, cache_dir=self.cache_dir,
            trust_remote_code=True, local_files_only=True,
        )
        self._model = AutoModelForCausalLM.from_pretrained(
            model_path, cache_dir=self.cache_dir,
            device_map="cpu", torch_dtype="auto",
            trust_remote_code=True, local_files_only=True)

    @property
    def cache_dir(self) -> str:
        return env.DIR_WEIGHTS

    def _load_checkpoint(self, load_path: Path):
        if is_checkpoint_deprecated(load_path):
            raise RuntimeError(f"Checkpoint {load_path} is old-style and not supported for lora merge")

        adapter_config = json.load(open(load_path / "adapter_config.json", 'r'))
        lora_cfg = {
            "lora_r": adapter_config["r"],
            "lora_alpha": adapter_config["lora_alpha"],
            "lora_dropout": adapter_config["lora_dropout"]
        }
        lora_target_modules = adapter_config["target_modules"]

        LoraMixin.apply_lora(
            self._model,
            lora_target_modules=lora_target_modules,
            **lora_cfg
        )

        finetune_weights = safetensors.torch.load_file(Path(load_path) / "adapter_model.safetensors")
        finetune_weights = {k.replace("base_model.model.", ""): v for k, v in finetune_weights.items()}
        missing, unexpected = self._model.load_state_dict(finetune_weights, strict=False)
        if len(unexpected) > 0:
            raise RuntimeError(f"Unexpected keys in finetune checkpoint: {unexpected}")

        embeddings_path = load_path / "new_embeddings.safetensors"
        if embeddings_path.exists():
            weights = safetensors.torch.load_file(str(embeddings_path))
            weights = {k.replace("base_model.model.", ""): v for k, v in weights.items()}
            missing, unexpected = self._model.load_state_dict(weights, strict=False)
            if len(unexpected) > 0:
                raise RuntimeError(f"Unexpected keys in finetune checkpoint: {unexpected}")

    def lora_patch_save(self, checkpoint_dir: Path, output_filename: Path):
        if output_filename.exists():
            raise FileExistsError(f"{output_filename} already exists")
        with tempfile.TemporaryDirectory() as tempdir:
            try:
                self._tokenizer.save_pretrained(tempdir)
                self._load_checkpoint(checkpoint_dir)
                patch_state_dict = LoraMixin.lora_patch_state_dict(self._model)
                LoraMixin.exclude_lora(self._model)
                self._model.load_state_dict(patch_state_dict, strict=False)
                self._model.save_pretrained(tempdir, safe_serialization=True)
                subprocess.check_call(
                    ["zip", "-q", "-0", "-r", str(output_filename.absolute()), "."],
                    cwd=tempdir)
            except Exception as e:
                raise RuntimeError(f"lora patch save failed: {e}")


if __name__ == "__main__":
    from argparse import ArgumentParser

    parser = ArgumentParser()
    parser.add_argument("model_path", type=str, help="Model path")
    parser.add_argument("checkpoint_path", type=Path, help="Path to checkpoint")
    parser.add_argument("output_filename", type=Path, help="Output filename")
    args = parser.parse_args()

    merger = LoraMerger(args.model_path)
    merger.lora_patch_save(args.checkpoint_path, args.output_filename)
