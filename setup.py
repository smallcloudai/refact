from setuptools import setup
from setuptools import find_packages


subdirs = [
    "code-contrast",
    "encoding-wrapper",
    # "known-models-db",
    # "models",
    # "scratchpads",
    # "self-hosting-machinery",
]

packages = []
package_dirs = {}

for sub in subdirs:
    for p in find_packages(sub):
        packages.append(p)
        package_dirs[p] = sub + "/" + p

setup(
    name="refact-self-hosting",
    py_modules=packages,
    packages=packages,
    package_dir=package_dirs,
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
