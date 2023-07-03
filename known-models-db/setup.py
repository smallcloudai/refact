from setuptools import setup
from setuptools import find_packages


setup(
    name="known-models-db",
    py_modules=["refact_known_models", "refact_toolbox_db"],
    packages=find_packages(),
    package_data={"refact_toolbox_db": ["refact_toolbox_db/htmls/*.html"]},
    version="0.0.1",
    install_requires=["code_contrast"]
)

