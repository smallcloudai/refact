#!/bin/bash

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
REPO_DIR=$(realpath $SCRIPT_DIR/../..)

ORIGIN_DOCKERFILE=${REPO_DIR}/Dockerfile
BASE_DOCKERFILE=${SCRIPT_DIR}/Dockerfile

BASE_IMAGE=$(cat $ORIGIN_DOCKERFILE | grep FROM | head -1 | awk '{print $2}')
echo FROM $BASE_IMAGE > $BASE_DOCKERFILE

echo ENV INSTALL_OPTIONAL=TRUE >> $BASE_DOCKERFILE
echo ENV FLASH_ATTENTION_FORCE_BUILD=TRUE >> $BASE_DOCKERFILE

echo RUN apt-get update >> $BASE_DOCKERFILE
echo 'RUN DEBIAN_FRONTEND="noninteractive" TZ=Etc/UTC apt-get install -y git python3 python3-pip python3-packaging && rm -rf /var/lib/{apt,dpkg,cache,log}' >> $BASE_DOCKERFILE

TORCH_STR=$(cat $ORIGIN_DOCKERFILE | grep torch==)

echo ${TORCH_STR} >> $BASE_DOCKERFILE
echo RUN pip install ninja >> $BASE_DOCKERFILE

REQUIREMENTS=$(cat $SCRIPT_DIR/requirements.txt)
IFS=$'\n' read -rd '' -a REQUIREMENTS <<<"$REQUIREMENTS"

for requirement in "${REQUIREMENTS[@]}"
do
    echo "RUN MAX_JOBS=8 pip install -v $requirement" >> $BASE_DOCKERFILE
done

docker buildx build --platform linux/amd64 -t ghcr.io/smallcloud/refact_base_image:latest --push -f $BASE_DOCKERFILE $REPO_DIR
