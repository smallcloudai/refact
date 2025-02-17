# CICL Workflows

### Processes
* Every PR will start a cicl build with primitive tests: checking dockerfile and packages. Hope in the future we will add tests and start them in this workflow 
* Every new commit(eg. merge pr) in the dev branch will start the nightly build. This workflow builds and releases a docker image with a tag nightly. We have only one nightly tag. It means nightly will be fresh always.
* Every new tag in the main branch will start the Release workflow. It’s similar to nightly, but the image tag will be the latest, and the git tag(ex. v1.3.0)

### Notes
Docker image has long-term processes like building cuda code and part of the docker image was built and cached in ghcr.io(https://github.com/orgs/smallcloudai/packages/container/package/refact_base_image). Dockerfile.base is a base image for all docker images.
Dockerfile includes a base image, so every developer can use a vanilla file for building locally.

GitHub actions will replace include directive to from ghcr.io/smallcloudai/refact_base_image:latest.

#### There are 2 scripts and file:
* .github/scripts/build_base_docker.sh - builder script for base container. This script builds Dockerfile.base and pushes it to ghcr.io.
* .github/scripts/cicl_docker_injection.sh - this script injects include directive in the root Dockerfile and changes into the cache version.
