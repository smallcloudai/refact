<p align='center'>
  <picture>
   <source width='300px' alt='White Refact Logo' media="(prefers-color-scheme: dark)" srcset="white-refact-logo.svg">
   <img width='300px' alt="Black Refact Logo" src="refact-logo.svg">
  </picture>
</p>

This repository contains the Refact WebUI, designed for fine-tuning and self-hosting of code models. You can seamlessly integrate these models into Refact plugins for enhanced code completion and chat capabilities.

---

[![Discord](https://img.shields.io/discord/1037660742440194089?logo=discord&label=Discord&link=https%3A%2F%2Fsmallcloud.ai%2Fdiscord)](https://smallcloud.ai/discord)
[![Twitter Follow](https://img.shields.io/twitter/follow/refact_ai)](https://twitter.com/intent/follow?screen_name=refact_ai)
![License](https://img.shields.io/github/license/smallcloudai/refact?cacheSeconds=1000)
[![Visual Studio](https://img.shields.io/visual-studio-marketplace/d/smallcloud.codify?label=VS%20Code)](https://marketplace.visualstudio.com/items?itemName=smallcloud.codify)
[![JetBrains](https://img.shields.io/jetbrains/plugin/d/com.smallcloud.codify?label=JetBrains)](https://plugins.jetbrains.com/plugin/20647-codify)

## Key Features 🌟

- ✅ Fine-tuning of open-source code models
- ✅ Self-hosting of open-source code models
- ✅ Download and upload Lloras
- ✅ Use models for code completion and chat inside Refact plugins
- ✅ Model sharding
- ✅ Host several small models on one GPU
- ✅ Use OpenAI and Anthropic keys to connect GPT models for chat

---

# Demo Video 🎥

This would be added soon

---

# Table of Contents 📚

- [Custom Installation](#custom-installation-%EF%B8%8F)
  - [Running Refact Self-Hosted in a Docker Container](#running-refact-self-hosted-in-a-docker-container-)
- [Getting Started with Plugins](#getting-started-with-plugins-)
- [Supported Models](#supported-models-)
- [Contributing](#contributing-)
- [Follow Us/FAQ](#follow-us-and-faq-)
- [License](#license-)


# Custom Installation ⚙️

You can also install refact repo without docker:
```shell
pip install .
```
If you have a GPU with CUDA capability >= 8.0, you can also install it with flash-attention v2 support:
```shell
FLASH_ATTENTION_FORCE_BUILD=TRUE MAX_JOBS=4 INSTALL_OPTIONAL=TRUE pip install .
```


## Running Refact Self-Hosted in a Docker Container 🐳


The easiest way to run the self-hosted server is a pre-build Docker image.

Install [Docker with NVidia GPU support](https://docs.nvidia.com/datacenter/cloud-native/container-toolkit/install-guide.html#docker).
On Windows you need to install WSL 2 first, [one guide to do this](https://docs.docker.com/desktop/install/windows-install).

Run docker container with following command:
```commandline
docker run -d --rm --gpus all --shm-size=256m -p 8008:8008 -v refact-perm-storage:/perm_storage smallcloud/refact_self_hosting:latest
```

`perm-storage` is a volume that is mounted inside the container. All the configuration files, downloaded weights and logs are stored here.

To upgrade the docker, delete it using `docker kill XXX` (the volume `perm-storage` will retain your
data), run `docker pull smallcloud/refact_self_hosting` and run it again.

Now you can visit http://127.0.0.1:8008 to see the server Web GUI.


<details><summary>Docker commands super short refresher</summary>
Add your yourself to docker group to run docker without sudo (works for Linux):

```commandline
sudo usermod -aG docker {your user}
```

List all containers:

```commandline
docker ps -a
```

Start and stop existing containers (stop doesn't remove them):

```commandline
docker start XXX
docker stop XXX
```

Shows messages from a container:
```commandline
docker logs -f XXX
```

Remove a container and all its data (except data inside a volume):
```commandline
docker rm XXX
```

Check out or delete a docker volume:
```commandline
docker volume inspect VVV
docker volume rm VVV
```
</details>


See [CONTRIBUTING.md](CONTRIBUTING.md) for installation without a docker container.

---

# Getting Started with Plugins 🔌


Download Refact for [VS Code](https://marketplace.visualstudio.com/items?itemName=smallcloud.codify) or [JetBrains](https://plugins.jetbrains.com/plugin/20647-refact-ai).

Go to plugin settings and set up a custom inference URL `http://127.0.0.1:8008`

<details><summary>JetBrains</summary>
Settings > Tools > Refact.ai > Advanced > Inference URL
</details>
<details><summary>VSCode</summary>
Extensions > Refact.ai Assistant > Settings > Infurl
</details>

---

## Supported Models 📊

| Model                                                                                             | Completion | Chat | Fine-tuning | [Deprecated](## "Will be removed in next versions") |
|---------------------------------------------------------------------------------------------------|------------|------|-------------|-----------------------------------------------------|
| [Refact/1.6B](https://huggingface.co/smallcloudai/Refact-1_6B-fim)                                | ✅          | ❌    | ✅           |                                                     |
| [starcoder2/3b/base](https://huggingface.co/bigcode/starcoder2-3b)                                | ✅          | ❌    | ✅           |                                                     |
| [starcoder2/7b/base](https://huggingface.co/bigcode/starcoder2-7b)                                | ✅          | ❌    | ✅           |                                                     |
| [starcoder2/15b/base](https://huggingface.co/bigcode/starcoder2-15b)                              | ✅          | ❌    | ✅           |      ✅                                              |
| [deepseek-coder/1.3b/base](https://huggingface.co/deepseek-ai/deepseek-coder-1.3b-base)           | ✅          | ❌    | ✅           |      ✅                                              |
| [deepseek-coder/5.7b/mqa-base](https://huggingface.co/deepseek-ai/deepseek-coder-5.7bmqa-base)    | ✅          | ❌    | ✅           |      ✅                                              |
| [magicoder/6.7b](https://huggingface.co/TheBloke/Magicoder-S-DS-6.7B-GPTQ)                        | ❌          | ✅    | ❌           |      ✅                                              |
| [mistral/7b/instruct-v0.1](https://huggingface.co/TheBloke/Mistral-7B-Instruct-v0.1-GPTQ)         | ❌          | ✅    | ❌           |      ✅                                              |
| [mixtral/8x7b/instruct-v0.1](https://huggingface.co/mistralai/Mixtral-8x7B-Instruct-v0.1)         | ❌          | ✅    | ❌           |                                                      |
| [deepseek-coder/6.7b/instruct](https://huggingface.co/TheBloke/deepseek-coder-6.7B-instruct-GPTQ) | ❌          | ✅    | ❌           |                                                      |
| [deepseek-coder/33b/instruct](https://huggingface.co/deepseek-ai/deepseek-coder-33b-instruct)     | ❌          | ✅    | ❌           |                                                      |
| [stable/3b/code](https://huggingface.co/stabilityai/stable-code-3b)                               | ✅          | ❌    | ❌           |                                                      |
| [llama3/8b/instruct](https://huggingface.co/meta-llama/Meta-Llama-3-8B-Instruct)                  | ❌          | ✅    | ❌           |                                                      |

---


# Contributing 🤝

If you wish to contribute to this project, feel free to explore our [current issues](https://github.com/smallcloudai/refact/issues) or open new issues related to (bugs/features) using our [CONTRIBUTING.md](CONTRIBUTING.md).


---

## Follow Us and FAQ ❓

**Q: Can I run a model on CPU?**

A: Currently, it doesn't run on CPU, but it's certainly possible to implement this.

- [Contributing](CONTRIBUTING.md)
- [Refact Docs](https://docs.refact.ai/guides/version-specific/self-hosted/)
- [GitHub Issues](https://github.com/smallcloudai/refact/issues) for bugs and errors
- [Community Forum](https://github.com/smallcloudai/refact/discussions) for community support and discussions
- [Discord](https://www.smallcloud.ai/discord) for chatting with community members
- [Twitter](https://twitter.com/refact_ai) for product news and updates

---

## License 📜

Refact is free to use for individuals and small teams under the BSD-3-Clause license. If you wish to use Refact for Enterprise, please [contact us](https://refact.ai/contact/).

