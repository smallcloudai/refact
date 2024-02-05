<p align='center'>
  <picture>
   <source width='300px' alt='White Refact Logo' media="(prefers-color-scheme: dark)" srcset="white-refact-logo.svg">
   <img width='300px' alt="Black Refact Logo" src="refact-logo.svg">
  </picture>
</p>

This repo consists Refact WebUI for fine-tuning and self-hosting of code models, that you can later use inside Refact plugins for code completion and chat.

---

[![Discord](https://img.shields.io/discord/1037660742440194089?logo=discord&label=Discord&link=https%3A%2F%2Fsmallcloud.ai%2Fdiscord)](https://smallcloud.ai/discord)
[![Twitter Follow](https://img.shields.io/twitter/follow/refact_ai)](https://twitter.com/intent/follow?screen_name=refact_ai)
![License](https://img.shields.io/github/license/smallcloudai/refact?cacheSeconds=1000)
[![Visual Studio](https://img.shields.io/visual-studio-marketplace/d/smallcloud.codify?label=VS%20Code)](https://marketplace.visualstudio.com/items?itemName=smallcloud.codify)
[![JetBrains](https://img.shields.io/jetbrains/plugin/d/com.smallcloud.codify?label=JetBrains)](https://plugins.jetbrains.com/plugin/20647-codify)

- [x] Fine-tuning of open-source code models
- [x] Self-hosting of open-source code models
- [x] Download and upload Lloras
- [x] Use models for code completion and chat inside Refact plugins
- [x] Model sharding
- [x] Host several small models on one GPU
- [x] Use OpenAI and Anthropic keys to connect GPT-models for chat

![self-hosting-refact](https://github.com/smallcloudai/refact/assets/5008686/18e48b42-b638-4606-bde0-cadd47fd26e7)

### Running Refact Self-Hosted in a Docker Container

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



### Setting Up Plugins


Download Refact for [VS Code](https://marketplace.visualstudio.com/items?itemName=smallcloud.codify) or [JetBrains](https://plugins.jetbrains.com/plugin/20647-refact-ai).

Go to plugin settings and set up a custom inference URL `http://127.0.0.1:8008`

<details><summary>JetBrains</summary>
Settings > Tools > Refact.ai > Advanced > Inference URL
</details>
<details><summary>VSCode</summary>
Extensions > Refact.ai Assistant > Settings > Infurl
</details>


## Supported models

| Model                                                                                             | Completion | Chat | Fine-tuning |
|---------------------------------------------------------------------------------------------------|------------|------|-------------|
| [Refact/1.6B](https://huggingface.co/smallcloudai/Refact-1_6B-fim)                                | +          |      | +           |
| [starcoder/1b/base](https://huggingface.co/smallcloudai/starcoderbase-1b)                         | +          |      | +           |
| [starcoder/3b/base](https://huggingface.co/smallcloudai/starcoderbase-3b)                         | +          |      | +           |
| [starcoder/7b/base](https://huggingface.co/smallcloudai/starcoderbase-7b)                         | +          |      | +           |
| [starcoder/15b/base](https://huggingface.co/TheBloke/starcoder-GPTQ)                              | +          |      |             |
| [starcoder/15b/plus](https://huggingface.co/TheBloke/starcoderplus-GPTQ)                          | +          |      |             |
| [wizardcoder/15b](https://huggingface.co/TheBloke/WizardCoder-15B-1.0-GPTQ)                       | +          |      |             |
| [codellama/7b](https://huggingface.co/TheBloke/CodeLlama-7B-fp16)                                 | +          |      | +           |
| [starchat/15b/beta](https://huggingface.co/TheBloke/starchat-beta-GPTQ)                           |            | +    |             |
| [wizardlm/7b](https://huggingface.co/TheBloke/WizardLM-7B-V1.0-Uncensored-GPTQ)                   |            | +    |             |
| [wizardlm/13b](https://huggingface.co/TheBloke/WizardLM-13B-V1.1-GPTQ)                            |            | +    |             |
| [wizardlm/30b](https://huggingface.co/TheBloke/WizardLM-30B-fp16)                                 |            | +    |             |
| [llama2/7b](https://huggingface.co/TheBloke/Llama-2-7b-Chat-GPTQ)                                 |            | +    |             |
| [llama2/13b](https://huggingface.co/TheBloke/Llama-2-13B-chat-GPTQ)                               |            | +    |             |
| [deepseek-coder/1.3b/base](https://huggingface.co/deepseek-ai/deepseek-coder-1.3b-base)           | +          |      | +           |
| [deepseek-coder/5.7b/mqa-base](https://huggingface.co/deepseek-ai/deepseek-coder-5.7bmqa-base)    | +          |      | +           |
| [magicoder/6.7b](https://huggingface.co/TheBloke/Magicoder-S-DS-6.7B-GPTQ)                        |            | +    |             |
| [mistral/7b/instruct-v0.1](https://huggingface.co/TheBloke/Mistral-7B-Instruct-v0.1-GPTQ)         |            | +    |             |
| [mixtral/8x7b/instruct-v0.1](https://huggingface.co/mistralai/Mixtral-8x7B-Instruct-v0.1)         |            | +    |             |
| [deepseek-coder/6.7b/instruct](https://huggingface.co/TheBloke/deepseek-coder-6.7B-instruct-GPTQ) |            | +    |             |
| [deepseek-coder/33b/instruct](https://huggingface.co/deepseek-ai/deepseek-coder-33b-instruct)     |            | +    |             |
| [stable/3b/code](https://huggingface.co/stabilityai/stable-code-3b)                               | +          |      |             |

## Usage

Refact is free to use for individuals and small teams under BSD-3-Clause license. If you wish to use Refact for Enterprise, please [contact us](https://refact.ai/contact/).

## Custom installation

You can also install refact repo without docker:
```shell
pip install .
```
If you have a GPU with CUDA capability >= 8.0, you can also install it with flash-attention v2 support:
```shell
FLASH_ATTENTION_FORCE_BUILD=TRUE MAX_JOBS=4 INSTALL_OPTIONAL=TRUE pip install .
```

## FAQ

Q: Can I run a model on CPU?

A: it doesn't run on CPU yet, but it's certainly possible to implement this.

## Community & Support

- Contributing [CONTRIBUTING.md](CONTRIBUTING.md)
- [GitHub issues](https://github.com/smallcloudai/refact/issues) for bugs and errors
- [Community forum](https://github.com/smallcloudai/refact/discussions) for community support and discussions
- [Discord](https://www.smallcloud.ai/discord) for chatting with community members
- [Twitter](https://twitter.com/refact_ai) for product news and updates
