from setuptools import setup, find_packages
import re
from typing import List
import platform


def refact_lsp_binary() -> List[str]:
    if platform.system() == "Windows":
        return ["refact/bin/refact-lsp.exe"]
    else:
        return ["refact/bin/refact-lsp"]


setup(
    name="refact",
    version="0.9.9",
    packages=find_packages(),
    install_requires=[
        "aiohttp",
        "termcolor",
        "pydantic",
        "prompt_toolkit",
        "requests",
        "pyyaml"
    ],
    author="Small Magellanic Cloud AI LTD",
    author_email="info@smallcloud.tech",
    description="A python client to refact-lsp server",
    url="https://github.com/smallcloudai/refact",
    classifiers=[
        "Topic :: Scientific/Engineering :: Artificial Intelligence",
        "Development Status :: 3 - Alpha",
        "Intended Audience :: Developers",
        "Programming Language :: Python :: 3",
        "License :: OSI Approved :: BSD License",
        "Operating System :: OS Independent",
        "Environment :: Console",
    ],
    python_requires=">=3.6",
    entry_points={
        'console_scripts': [
            'refact=refact.cmdline_main:entrypoint',
        ],
    },
    include_package_data=True,
    data_files=[('bin', refact_lsp_binary())]
)


# XXX: move to automatic build
#   sync version from Cargo.toml
#   python setup.py sdist
#   twine upload --repository pypi dist/refact-0.9.7.tar.gz

# XXX: installing per platform, unclear so far
# export CIBW_SKIP="cp27-manylinux_* cp34-manylinux_* cp35-manylinux_* cp36-manylinux_* cp37-manylinux_* cp38-manylinux_* cp39-manylinux_* cp310-manylinux_* cp311-manylinux_* cp27-macosx_* cp34-macosx_* cp35-macosx_* cp36-macosx_* cp37-macosx_* cp39-macosx_* cp310-macosx_* cp311-macosx_* cp27-win_* cp34-win_* cp35-win_* cp36-win_* cp37-win_* cp38-win_* cp39-win_* cp310-win_* cp311-win_* cp312-maxosx_* cp312-macosx_* cp313-macosx_* pp*"
# cibuildwheel --output-dir wheelhouse
