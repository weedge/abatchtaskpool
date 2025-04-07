#!/usr/bin/env python
from setuptools import find_packages, setup

setup(
    name="abatchtaskpool",
    description="Python batch task pool with asyncio support",
    long_description=open("README.md").read().strip(),
    long_description_content_type="text/markdown",
    version="0.1.0",
    license="BSD 3-Clause",
    url="https://github.com/weedge/abatchtaskpool",
    author="weedge",
    author_email="weege007@gmail.com",
    python_requires=">=3.10",
    packages=find_packages(),
    include_package_data=True,
    install_requires=[],
    project_urls={
        "Changes": "https://github.com/weedge/abatchtaskpool/releases",
        "Code": "https://github.com/weedge/abatchtaskpool",
        "Issue tracker": "https://github.com/weedge/abatchtaskpool/issues",
    },
    keywords=["batch", "task", "pool", "asyncio"],
)
