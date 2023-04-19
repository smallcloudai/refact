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

RUN pip install --no-cache-dir torch==1.13.1+cu116 --extra-index-url https://download.pytorch.org/whl/cu116
RUN pip install --no-cache-dir IPython numpy tokenizers tiktoken fastapi hypercorn termcolor cdifflib
RUN pip install --no-cache-dir cloudpickle dataclasses_json huggingface_hub blobfile
RUN pip install --no-cache-dir git+https://github.com/smallcloudai/code-contrast.git
RUN pip install --no-cache-dir git+https://github.com/smallcloudai/refact-self-hosting.git

RUN mkdir /workdir
ENV SERVER_WORKDIR=/workdir
ENV SERVER_PORT=8008
EXPOSE $SERVER_PORT

CMD ["python", "-m", "refact_self_hosting.watchdog", "--workdir", "/workdir"]
