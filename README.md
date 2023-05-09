<div align="center">

# <font color="red">[{</font> Refact.ai Inference Server

</div>

This is a self-hosted server for the [refact.ai](https://www.refact.ai) coding assistant.

With Refact you can run high-quality AI code completions on-premise and use a number of
functions for code transformation and ask questions in the chat.

This server allows you to run AI coding models on your hardware, your code doesn't go outside your control.

At the moment, you can choose between following models:

| Model                     | GPU (VRAM) | CPU (RAM) | Completion | AI Toolbox | Chat |
| ------------------------- | ---------- | --------- | ---------- | ---------- | ---- |
| CONTRASTcode/medium/multi |        3Gb |       3Gb |          + |            |      |    
| CONTRASTcode/3b/multi     |        8Gb |      12Gb |          + |            |      |
| starcoder/15b/base4bit    |       12Gb |         - |          + |          + |    + |
| starcoder/15b/base8bit    |       24Gb |         - |          + |          + |    + |

Refact is currently available as a plugin for [JetBrains](https://plugins.jetbrains.com/plugin/20647-refact-ai)
products and [VS Code IDE](https://marketplace.visualstudio.com/items?itemName=smallcloud.codify).


## Demo

<table align="center">
<tr>
<th><img src="https://plugins.jetbrains.com/files/20647/screenshot_277b57c5-2104-4ca8-9efc-1a63b8cb330f" align="center"/></th>
</tr>
</table>

## Getting started
Install plugin for your IDE:
[JetBrains](https://plugins.jetbrains.com/plugin/20647-refact-ai) or
[VSCode](https://marketplace.visualstudio.com/items?itemName=smallcloud.codify).


### Running Server in Docker
The recommended way to run server is a pre-build Docker image.

Install [Docker with NVidia GPU support](https://docs.nvidia.com/datacenter/cloud-native/container-toolkit/install-guide.html#docker).
On Windows you need to install WSL 2 first, [one guide to do this](https://docs.docker.com/desktop/install/windows-install).


<details><summary>Docker tips & tricks</summary>

Add your yourself to docker group to run docker without sudo (works for Linux):
```commandline
sudo usermod -aG docker {your user}
```
List all containers:
```commandline
docker ps -a
```
Create a new container:
```commandline
docker run
```
Start and stop existing containers (stop doesn't remove them):
```commandline
docker start
docker stop
```
Remove a container and all its data:
```commandline
docker rm
```

Shows messages from the container:
```commandline
docker logs -f
```
</details>

Choose model from available ones.

Run docker container with following command:
```commandline
docker run --rm --gpus 0 -p 8008:8008 -v refact_workdir:/workdir --env SERVER_MODEL=<model name> smallcloud/refact_self_hosting
```
If you don't have a suitable GPU run it on CPU:
```commandline
docker run --rm -p 8008:8008 -v refact_workdir:/workdir --env SERVER_MODEL=<model name> smallcloud/refact_self_hosting
```
After start container will automatically download the chosen model.


### Running Manually

To run server manually, install this repo first (this might install a lot of packages on your computer):
```commandline
pip install git+https://github.com/smallcloudai/code-contrast.git
pip install git+https://github.com/smallcloudai/refact-self-hosting.git
```
Now you can run server with following command:
```commandline
python -m refact_self_hosting.server --workdir /workdir --model <model name>
```


## Setting Up Plugins

Go to plugin settings and set up a custom inference url:
```commandline
https://localhost:8008
```
<details><summary>JetBrains</summary>
Settings > Tools > Refact.ai > Advanced > Inference URL
</details>
<details><summary>VSCode</summary>
Extensions > Refact.ai Assistant > Settings > Infurl
</details>


Now it should work, just try to write some code! If it doesn't, please report your experience to
[GitHub issues](https://github.com/smallcloudai/code-contrast/issues).


<details><summary>Remote server</summary>

If you run server on remote host, you should add it to /etc/hosts
(or C:\Windows\System32\drivers\etc\hosts on Windows) on client.
Do not forget to replace {server ip address} to real server ip address.

```commandline
{server ip address}  inference.smallcloud.local
```

and set up this inference url in plugin:

```commandline
https://inference.smallcloud.local:8008
```
</details>


## Community & Support
Join our [Discord server](https://www.smallcloud.ai/discord) and follow our
[Twitter](https://twitter.com/refact_ai) to get the latest updates.



## Contributing

We are open for contributions. If you have any ideas and ready to implement this, just:
- make a [fork](https://github.com/smallcloudai/code-contrast/fork)
- make your changes, commit to your fork
- and open a [PR](https://github.com/smallcloudai/code-contrast/fork)
