from setuptools import setup, find_packages
import re

setup(
    name="refact",
    version="0.9.9",
    packages=find_packages(),
    install_requires=[
        "aiohttp",
        "termcolor",
        "pydantic",
        "prompt_toolkit",
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
            'refact=refact.refact_cmdline:cmdline_main',
        ],
    },
)


# XXX: move to automatic build
#   sync version from Cargo.toml
#   python setup.py sdist
#   twine upload --repository pypi dist/refact-0.9.7.tar.gz
