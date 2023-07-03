from setuptools import setup, find_packages

setup(
    name="no-gpu-scratchpads",
    py_modules=[
        "async_scratchpad",
        "codex_toolbox",
        "gpt_toolbox",
        "no_gpu_scratchpads",
        "longthink_db",
    ],
    package_data={
        "codex_toolbox": ["misc/*", "prompts/*"],
    },
    version="0.0.1",
    packages=find_packages(),
    install_requires=[
        "tiktoken==0.4.0",
        "openai",
    ],
)
