FROM nvidia/cuda:11.8.0-cudnn8-devel-ubuntu20.04

RUN apt-get update
RUN DEBIAN_FRONTEND="noninteractive" apt-get install -y \
    curl \
    git \
    htop \
    tmux \
    vim \
    mpich \
    libmpich-dev \
    python3 python3-pip \
    && rm -rf /var/lib/{apt,dpkg,cache,log}

RUN echo "export PATH=/usr/local/cuda/bin:\$PATH" > /etc/profile.d/50-smc.sh
RUN update-alternatives --install /usr/bin/python python /usr/bin/python3 1

ARG TARGETARCH
RUN if [ "$TARGETARCH" = "amd64" ]; then \
      pip install --no-cache-dir torch==2.0.1 --index-url https://download.pytorch.org/whl/cu118; \
    elif [ "$TARGETARCH" = "arm64" ]; then \
      pip install --no-cache-dir torch==1.13.1; \
    else \
      exit 1; \
    fi

ENV TORCH_CUDA_ARCH_LIST="6.1;7.0;7.5;8.0;8.6+PTX"
RUN if [ "$TARGETARCH" = "amd64" ]; then \
      BUILD_QUANT_CUDA=1 pip install --no-cache-dir git+https://github.com/smallcloudai/code-contrast.git@lora; \
    elif [ "$TARGETARCH" = "arm64" ]; then \
      pip install --no-cache-dir git+https://github.com/smallcloudai/code-contrast.git@lora; \
    else \
      exit 1; \
    fi

RUN pip install git+https://github.com/smallcloudai/no-gpu-scratchpads.git@self_hosting
RUN pip install git+https://github.com/smallcloudai/smallcloud.git

COPY . /tmp/app
RUN pip install /tmp/app && rm -rf /tmp/app

ENV REFACT_PERM_DIR "/perm_storage"
ENV REFACT_TMP_DIR "/tmp"

EXPOSE 8008

CMD ["python", "-m", "refact_watchdog.docker_watchdog"]
