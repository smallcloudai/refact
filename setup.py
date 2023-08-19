from setuptools import setup

setup(
    version="0.0.1",
    url="https://github.com/smallcloudai/refact_lsp",
    summary="LSP server for Refact, suitable for Sublime Text, and other editors",
    description="Install, run refact_lsp, enter your custom server URL, or just an API Key",
    license='BSD 3-Clause License',
    install_requires=[
        "requests",
    ],
    author="Small Magellanic Cloud AI Ltd.",
    author_email="info@smallcloud.tech",
    entry_points={
        "console_scripts": ["refact_lsp = refact_lsp.__main__:main"],
    },
    classifiers=[
        "Development Status :: 3 - Alpha",
        "Topic :: Scientific/Engineering :: Artificial Intelligence",
        "Intended Audience :: Developers",
        "License :: OSI Approved :: BSD License",
        "Environment :: Console",
        "Operating System :: OS Independent",
    ]
)
