from setuptools import setup
from setuptools import find_packages

setup(
    name="refact_encoding",
    py_modules=["refact_encoding"],
    packages=find_packages(),
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
