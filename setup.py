from setuptools import setup, find_packages

setup(
    name="refact",
    version="0.1",
    packages=find_packages(),
    install_requires=[
        "aiohttp",
        "termcolor",
        "pydantic",
    ],
    author="Small Magellanic Cloud AI LTD",
    author_email="info@smallcloud.tech",
    description="A python client to refact-lsp server",
    url="https://github.com/smallcloudai/refact",
    classifiers=[
        "Programming Language :: Python :: 3",
        "License :: OSI Approved :: BSD 3-Clause License",
        "Operating System :: OS Independent",
    ],
    python_requires=">=3.6",
)
