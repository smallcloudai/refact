import os

from copy import copy
from dataclasses import dataclass
from dataclasses import field
from setuptools import setup, find_packages

from typing import List, Set


@dataclass
class PyPackage:
    requires: List[str] = field(default_factory=list)
    requires_packages: List[str] = field(default_factory=list)
    data: List[str] = field(default_factory=list)


all_refact_packages = {
    "code_contrast": PyPackage(
        requires=["cdifflib", "termcolor", "numpy", "dataclasses"],
        requires_packages=["refact_encoding"]),
    "known_models_db": PyPackage(
        requires=["dataclasses", "dataclasses_json"],
        data=["refact_toolbox_db/htmls/*.html"]),
    "refact_encoding": PyPackage(
        requires=["tiktoken", "tokenizers==0.14.0", "sentencepiece", "termcolor"],
        data=["*.json"]),
    "refact_models": PyPackage(
        requires=["torch", "blobfile", "cloudpickle", "fsspec==2023.1.0", "huggingface_hub==0.16.4",
                  "transformers", "dataclasses", "dataclasses_json"],
        requires_packages=["refact_encoding"]),
    "refact_scratchpads": PyPackage(
        requires=["termcolor", "torch"],
        requires_packages=["refact_encoding", "code_contrast", "refact_scratchpads_no_gpu"]),
    "refact_scratchpads_no_gpu": PyPackage(
        requires=["termcolor", "aiohttp", "tiktoken", "openai", "ujson", "setproctitle"]),
    "refact_data_pipeline": PyPackage(
        requires=["numpy", "tokenizers==0.14.0", "torch", "requests", "cloudpickle",
                  "tqdm", "dataclasses_json", "termcolor", 'more_itertools',
                  "cdifflib", "ujson", "zstandard", "scipy", "deepspeed==0.9.2",
                  "peft", "einops", "mpi4py", "matplotlib", "transformers",
                  "triton==2.0.0.post1", "torchinfo", "giturlparse", "jsonlines",
                  "binpacking", "filelock", "tables==3.8.0", "pygments", "kshingle"],
        requires_packages=["refact_encoding", "code_contrast", "self_hosting_machinery",
                           "refact_models"],
        data=["git_command.exp"],
    ),
    "self_hosting_machinery": PyPackage(
        requires=["aiohttp", "cryptography", "fastapi==0.100.0", "giturlparse", "pydantic==1.10.10",
                  "starlette==0.27.0", "uvicorn", "uvloop", "python-multipart", "auto-gptq==0.4.2",
                  "torch", "transformers", "termcolor", "accelerate", "bitsandbytes"],
        requires_packages=["refact_scratchpads", "refact_scratchpads_no_gpu", "refact_models",
                           "known_models_db", "refact_data_pipeline"],
        data=["webgui/static/*", "webgui/static/js/*", "webgui/static/components/modals/*", "watchdog/watchdog.d/*"]),
}


def find_required_packages(packages: Set[str]) -> Set[str]:
    updated_packages = copy(packages)
    for name in packages:
        assert name in all_refact_packages, f"Package {name} not found in repo"
        updated_packages.update(all_refact_packages[name].requires_packages)
    if updated_packages != packages:
        return find_required_packages(updated_packages)
    return packages


setup_package = os.environ.get("SETUP_PACKAGE", None)
if setup_package is not None:
    if setup_package not in all_refact_packages:
        raise ValueError(f"Package {setup_package} not found in repo")
    setup_packages = {
        name: py_package
        for name, py_package in all_refact_packages.items()
        if name in find_required_packages({setup_package})
    }
else:
    setup_packages = all_refact_packages


setup(
    name="refact-self-hosting",
    version="1.0.0",
    py_modules=list(setup_packages.keys()),
    package_data={
        name: py_package.data
        for name, py_package in setup_packages.items()
        if py_package.data
    },
    packages=find_packages(include=(
        f"{name}*" for name in setup_packages
    )),
    install_requires=list({
        required_package
        for py_package in setup_packages.values()
        for required_package in py_package.requires
    }),
)
