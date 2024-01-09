#/bin/bash

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
REPO_DIR=$(realpath $SCRIPT_DIR/../..)

ORIGIN_DOCKERFILE=${REPO_DIR}/Dockerfile
BASE_IMAGE=$(cat $ORIGIN_DOCKERFILE | grep FROM | head -1 | awk '{print $2}')

CACHE_IMAGE="ghcr.io/smallcloud/refact_base_image:latest"

sed -i "s!${BASE_IMAGE}!${CACHE_IMAGE}!" ${ORIGIN_DOCKERFILE}

