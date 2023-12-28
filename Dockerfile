FROM nvidia/cuda:11.8.0-cudnn8-devel-ubuntu22.04

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
RUN pip install --no-cache-dir torch==2.1.2 --index-url https://download.pytorch.org/whl/cu118
# auto-gptq
RUN pip install auto-gptq==0.6.0 --extra-index-url https://huggingface.github.io/autogptq-index/whl/cu118/

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

RUN DEBIAN_FRONTEND=noninteractive TZ=Etc/UTC apt-get install -y python3-packaging

# cassandra
RUN apt-get install -y \
    default-jdk \
    wget \
    sudo
RUN echo "deb https://debian.cassandra.apache.org 41x main" | tee -a /etc/apt/sources.list.d/cassandra.sources.list
RUN curl https://downloads.apache.org/cassandra/KEYS | apt-key add -
RUN apt-get update
RUN apt-get install cassandra -y

# refact lsp requisites
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | bash -s -- -y
ENV PATH="${PATH}:/root/.cargo/bin"
RUN git clone https://github.com/smallcloudai/refact-lsp.git /tmp/refact-lsp \
    && cd /tmp/refact-lsp \
    && cargo install --path . \
    && rm -rf /tmp/refact-lsp

ENV INSTALL_OPTIONAL=TRUE
ENV FLASH_ATTENTION_FORCE_BUILD=TRUE
ENV MAX_JOBS=8
COPY . /tmp/app
RUN pip install ninja
RUN pip install /tmp/app -v --no-build-isolation && rm -rf /tmp/app

ENV REFACT_PERM_DIR "/perm_storage"
ENV REFACT_TMP_DIR "/tmp"
ENV RDMAV_FORK_SAFE 0
ENV RDMAV_HUGEPAGES_SAFE 0

EXPOSE 8008

COPY docker-entrypoint.sh /
RUN chmod +x docker-entrypoint.sh

CMD ./docker-entrypoint.sh
