from setuptools import setup, find_packages


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
        "numpy", "torch", "termcolor", "dataclasses_json", "dataclasses", "tiktoken",
        # code_contrast
        "cdifflib",
        # refact_encoding
        "tokenizers", "sentencepiece",
        # refact_scratchpads_no_gpu
        "openai", "ujson",
        # refact_models
        "blobfile", "cloudpickle", "huggingface_hub", "transformers",
        # self_hosting_machinery
        "aiohttp", "cryptography", "fastapi", "giturlparse", "pydantic",
        "starlette", "uvicorn", "uvloop", "python-multipart", "auto-gptq",
    ],
)
