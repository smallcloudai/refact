#!/bin/bash

SCRIPT_DIR=$( dirname "$0" )
REPO_DIR=$(realpath $SCRIPT_DIR/../..)

BASE_DOCKERFILE=${REPO_DIR}/Dockerfile.base

docker buildx build --platform linux/amd64 -t ghcr.io/smallcloudai/refact_base_image:latest --push -f $BASE_DOCKERFILE $REPO_DIR --no-cache
