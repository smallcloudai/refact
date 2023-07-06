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
    ],
    package_data={
        "known_models_db": ["refact_toolbox_db/htmls/*.html"],
        "refact_encoding": ["*.json"],
        "self_hosting_machinery": ["webgui/static/*", "webgui/static/js/*", "watchdog/watchdog.d/*"],
    },
    packages=find_packages(),
    install_requires=[
        # self_hosting_machinery
        "fastapi", "uvloop", "uvicorn", "aiohttp", "python-multipart", "smallcloud", "blobfile",
        # known models
        "dataclasses_json", "termcolor",
        # encoding
        "tiktoken",
        # code contrast
        "cdifflib",
        # models
        "transformers", "torch",
    ],
    **additional_setup_kwargs,
)
