from setuptools import setup
from setuptools import find_packages

setup(
    name="encoding-wrapper",
    py_modules=["refact_encoding"],
    packages=find_packages(),
    package_data={"refact_encoding": ["*.json", "*.tokenizer.model"]},
    version="0.9.0",
    install_requires=[
        "numpy",
        "termcolor",
        "cdifflib",
        "tokenizers",
        "sentencepiece",
        "tiktoken",
        ],
)
