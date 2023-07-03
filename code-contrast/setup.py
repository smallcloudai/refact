from setuptools import setup
from setuptools import find_packages


setup(
    name="code-contrast",
    py_modules=["code_contrast"],
    packages=find_packages(),
    package_data={"code_contrast": ["encoding/*.json", "model_caps/htmls/*.html"]},
    version="0.0.3",
    install_requires=[
        "numpy",
        "termcolor",
        "cdifflib",
        "tokenizers",
        "sentencepiece",
        "tiktoken",
        ],
)
