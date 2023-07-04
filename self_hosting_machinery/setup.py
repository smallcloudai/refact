from setuptools import setup, find_packages


setup(
    name="self-hosting-machinery",
    version="0.9.0",
    py_modules=["refact_inference", "refact_scripts", "refact_watchdog", "refact_webgui"],
    packages=find_packages(),
    install_requires=[
    ],
)
