from setuptools import setup
from setuptools import find_packages


setup(
    name="refact-self-hosting",
    py_modules=["refact_self_hosting"],
    packages=find_packages(),
    package_data={"refact_self_hosting": ["webgui/static/*", "webgui/static/js/*", "watchdog.d/*"]},
    version="0.0.4",
    install_requires=["numpy", "tokenizers", "fastapi", "uvicorn", "termcolor",
                      "jsonlines",
                      "huggingface_hub", "tiktoken", "cdifflib", "cloudpickle",
                      "sentencepiece", "dataclasses_json", "torch", "transformers",
                      "blobfile", "cryptography",
                      "smallcloud", "code_contrast", "uvloop", "aiohttp", "python-multipart",
                      "bitsandbytes",
                      ],
)

