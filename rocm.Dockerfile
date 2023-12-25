FROM ocelot88/rocm-pytorch-slim:rocm-5.7.1-dev-torch-2.3
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


RUN update-alternatives --install /usr/bin/python python /usr/bin/python3 1

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

ENV INSTALL_OPTIONAL=TRUE
ENV BUILD_CUDA_EXT=1
ENV USE_ROCM=1
ENV GITHUB_ACTIONS=true
ENV AMDGPU_TARGETS="gfx1030"
ENV FLASH_ATTENTION_FORCE_BUILD=TRUE
ENV MAX_JOBS=8
COPY . /tmp/app
RUN pip install --upgrade pip ninja packaging
RUN DEBIAN_FRONTEND=noninteractive apt-get install python3-mpi4py -y
ENV PYTORCH_ROCM_ARCH="gfx1030"
ENV ROCM_TARGET="gfx1030"
ENV ROCM_HOME=/opt/rocm-5.7.1
# TODO: https://github.com/TimDettmers/bitsandbytes/pull/756 remove this layer, when this pr merged
RUN git clone https://github.com/arlo-phoenix/bitsandbytes-rocm-5.6 && \
    cd bitsandbytes-rocm-5.6 && \
    make hip && pip install . && \
    cd .. && rm -rf bitsandbytes-rocm-5.6
RUN pip install /tmp/app -v --no-build-isolation && rm -rf /tmp/app
RUN ln -s ${ROCM_HOME} /opt/rocm
ENV REFACT_PERM_DIR "/perm_storage"
ENV REFACT_TMP_DIR "/tmp"
ENV RDMAV_FORK_SAFE 0
ENV RDMAV_HUGEPAGES_SAFE 0

EXPOSE 8008

CMD ["python", "-m", "self_hosting_machinery.watchdog.docker_watchdog"]
