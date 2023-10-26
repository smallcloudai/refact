FROM nvidia/cuda:12.1.1-cudnn8-devel-ubuntu22.04

RUN apt-get update
RUN DEBIAN_FRONTEND="noninteractive" apt-get install -y \
    curl \
    git \
    htop \
    tmux \
    file \
    vim \
    expect \
    mpich \
    libmpich-dev \
    python3 python3-pip \
    && rm -rf /var/lib/{apt,dpkg,cache,log}

RUN echo "export PATH=/usr/local/cuda/bin:\$PATH" > /etc/profile.d/50-smc.sh
RUN update-alternatives --install /usr/bin/python python /usr/bin/python3 1

# torch
RUN pip install --no-cache-dir torch==2.1.0 --index-url https://download.pytorch.org/whl/cu121

# linguist requisites
RUN apt-get update
RUN DEBIAN_FRONTEND=noninteractive TZ=Etc/UTC apt-get install -y \
    expect \
    ruby-full \
    ruby-bundler \
    build-essential \
    cmake \
    pkg-config \
    libicu-dev \
    zlib1g-dev \
    libcurl4-openssl-dev \
    libssl-dev
RUN git clone https://github.com/smallcloudai/linguist.git /tmp/linguist \
    && cd /tmp/linguist \
    && bundle install \
    && rake build_gem
ENV PATH="${PATH}:/tmp/linguist/bin"

ENV BUILD_CUDA_EXT=1
ENV GITHUB_ACTIONS=true
ENV TORCH_CUDA_ARCH_LIST="6.0;6.1;7.0;7.5;8.0;8.6;8.9;9.0+PTX"
COPY . /tmp/app
RUN pip install /tmp/app && rm -rf /tmp/app

ENV REFACT_PERM_DIR "/perm_storage"
ENV REFACT_TMP_DIR "/tmp"

EXPOSE 8008

CMD ["python", "-m", "self_hosting_machinery.watchdog.docker_watchdog"]
