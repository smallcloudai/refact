FROM nvidia/cuda:11.6.2-cudnn8-runtime-ubuntu20.04

RUN apt-get update
RUN DEBIAN_FRONTEND="noninteractive" apt-get install -y \
    curl \
    git \
    htop \
    tmux \
    vim \
    python3 python3-pip \
    && rm -rf /var/lib/{apt,dpkg,cache,log}

RUN echo "export PATH=/usr/local/cuda/bin:\$PATH" > /etc/profile.d/50-smc.sh
RUN update-alternatives --install /usr/bin/python python /usr/bin/python3 1

ARG TARGETARCH
RUN if [ "$TARGETARCH" = "amd64" ]; then \
      pip install --no-cache-dir torch==1.13.1+cu116 --extra-index-url https://download.pytorch.org/whl/cu116; \
    elif [ "$TARGETARCH" = "arm64" ]; then \
      pip install --no-cache-dir torch==1.13.1; \
    else \
      exit 1; \
    fi


# ADD "https://www.random.org/cgi-bin/randbyte?nbytes=10&format=h" skipcache

COPY . /tmp/refact-self-hosting
RUN pip install --no-cache-dir --requirement /tmp/refact-self-hosting/requirements.txt
RUN pip install /tmp/refact-self-hosting

RUN mkdir /workdir
ENV SERVER_WORKDIR=/workdir
ENV SERVER_PORT=8008
EXPOSE $SERVER_PORT

CMD ["python", "-m", "refact_self_hosting.watchdog", "--workdir", "/workdir"]
