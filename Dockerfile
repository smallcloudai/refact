# syntax = devthefuture/dockerfile-x

INCLUDE Dockerfile.base

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
    protobuf-compiler \
    python3 python3-pip \
    && rm -rf /var/lib/{apt,dpkg,cache,log}

RUN echo "export PATH=/usr/local/cuda/bin:\$PATH" > /etc/profile.d/50-smc.sh
RUN update-alternatives --install /usr/bin/python python /usr/bin/python3 1

# auto-gptq
RUN pip install auto-gptq==0.6.0 --extra-index-url https://huggingface.github.io/autogptq-index/whl/cu118/

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
RUN git clone https://github.com/smallcloudai/refact-lsp.git /tmp/refact-lsp
RUN echo "refact-lsp $(git -C /tmp/refact-lsp rev-parse HEAD)" >> /refact-build-info.txt
RUN cd /tmp/refact-lsp \
    && cargo install --path . \
    && rm -rf /tmp/refact-lsp

COPY . /tmp/app
RUN echo "refact $(git -C /tmp/app rev-parse HEAD)" >> /refact-build-info.txt
RUN pip install ninja
RUN pip install /tmp/app -v --no-build-isolation && rm -rf /tmp/app

ENV REFACT_PERM_DIR "/perm_storage"
ENV REFACT_TMP_DIR "/tmp"
ENV RDMAV_FORK_SAFE 0
ENV RDMAV_HUGEPAGES_SAFE 0

EXPOSE 8008

COPY database-start.sh /
RUN chmod +x database-start.sh
COPY docker-entrypoint.sh /
RUN chmod +x docker-entrypoint.sh

CMD ./docker-entrypoint.sh
