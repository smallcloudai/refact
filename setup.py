from setuptools import setup, find_packages

setup(
    name="refact",
    version="0.8.2",
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
        "Topic :: Scientific/Engineering :: Artificial Intelligence",
        "Development Status :: 3 - Alpha",
        "Intended Audience :: Developers",
        "Programming Language :: Python :: 3",
        "License :: OSI Approved :: BSD License",
        "Operating System :: OS Independent",
        "Environment :: Console",
    ],
    python_requires=">=3.6",
)
