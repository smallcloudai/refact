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

RUN pip install --no-cache-dir torch==2.0.1 --index-url https://download.pytorch.org/whl/cu118
RUN pip install --no-cache-dir git+https://github.com/smallcloudai/smallcloud.git

ENV TORCH_CUDA_ARCH_LIST="6.1;7.0;7.5;8.0;8.6+PTX"
COPY . /tmp/app
RUN pip install /tmp/app && rm -rf /tmp/app

ENV REFACT_PERM_DIR "/perm_storage"
ENV REFACT_TMP_DIR "/tmp"

EXPOSE 8008

CMD ["python", "-m", "self_hosting_machinery.watchdog.docker_watchdog"]
