#/bin/bash

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
REPO_DIR=$(realpath $SCRIPT_DIR/../..)

ORIGIN_DOCKERFILE=${REPO_DIR}/Dockerfile

CACHE_IMAGE="FROM ghcr.io/smallcloudai/refact_base_image:latest"

sed -i "s!INCLUDE Dockerfile.base!${CACHE_IMAGE}!" ${ORIGIN_DOCKERFILE}

