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
        "self_hosting_machinery": ["webgui/static/*", "webgui/static/js/*", "watchdog/watchdog.d/*"],
        "known_models_db": ["refact_toolbox_db/htmls/*"],
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
)
