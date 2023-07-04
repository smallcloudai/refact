from setuptools import setup
from setuptools import find_packages

setup(
    name="refact-known-models-db",
    py_modules=["refact_known_models", "refact_scratchpads", "refact_scratchpads_no_gpu", "refact_toolbox_db"],
    packages=find_packages(),
    package_data={"refact_toolbox_db": ["htmls/*.html"]},
    version="0.9.0",
    install_requires=[
    ],
)
