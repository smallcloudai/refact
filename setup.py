import os
from setuptools import setup, find_packages


additional_setup_kwargs = dict()
if os.environ.get("BUILD_QUANT_CUDA", "0") == "1":
    try:
        import torch
        from torch.utils import cpp_extension
        additional_setup_kwargs = {
            "ext_modules": [
                cpp_extension.CUDAExtension("quant_cuda", [
                    "quant_cuda/quant_cuda.cpp",
                    "quant_cuda/quant_cuda_kernel.cu"
                ])
            ],
            "cmdclass": {"build_ext": cpp_extension.BuildExtension},
        }
    except ImportError:
        print("To build quant_cuda extension install torch")


setup(
    name="refact-self-hosting",
    version="0.9.0",
    py_modules=[
        "code_contrast",
        "refact_encoding",
        "known_models_db",
        "refact_scratchpads",
        "refact_scratchpads_no_gpu",
        "refact_models",
        "self_hosting_machinery",
        "refact_vecdb"
    ],
    package_data={
        "known_models_db": ["refact_toolbox_db/htmls/*.html"],
        "refact_encoding": ["*.json"],
        "self_hosting_machinery": ["webgui/static/*", "webgui/static/js/*", "watchdog/watchdog.d/*"],
    },
    packages=find_packages(),
    install_requires=[
        "numpy", "torch", "termcolor", "smallcloud", "dataclasses_json", "dataclasses", "tiktoken",
        # code_contrast
        "cdifflib",
        # refact_encoding
        "tokenizers", "sentencepiece",
        # refact_scratchpads_no_gpu
        "openai>=0.27.8", "ujson", "aiohttp", "requests",
        # refact_models
        "blobfile", "cloudpickle", "huggingface_hub", "transformers",
        # self_hosting_machinery
        "aiohttp", "cryptography", "fastapi", "giturlparse", "pydantic",
        "starlette", "uvicorn", "uvloop", "python-multipart",
        # refact_vecdb
        "cassandra-driver", "pynndescent",
        "tqdm", "numpy", "pydantic",
        "fastapi", "uvicorn", "uvloop",
        "more-itertools", "tqdm", "requests",
        "aiohttp", "ujson"
    ],
    **additional_setup_kwargs,
)
