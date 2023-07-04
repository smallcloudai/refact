from setuptools import setup, find_packages


setup(
    name="refact-self-hosting",
    version="0.9.0",
    py_modules=[
        "code_contrast",
        "refact_encoding",
        "known_models_db",
        "refact_models",
        "self_hosting_machinery",
    ],
    packages=find_packages(),
    install_requires=[
    ],
)
