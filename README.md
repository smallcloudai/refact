<div align="center">

# <font color="red">[{</font> Refact.ai Inference Server
### Self-hosted server for [Refact.ai](https://www.refact.ai) coding assistant.

</div>


## Demo

<table align="center">
<tr>
<th><img src="https://plugins.jetbrains.com/files/20647/screenshot_277b57c5-2104-4ca8-9efc-1a63b8cb330f" align="center"/></th>
</tr>
</table>


## Features
* Plugins for [JetBrains](https://plugins.jetbrains.com/plugin/20647-refact-ai) products and
  [VSCode IDE](https://marketplace.visualstudio.com/items?itemName=smallcloud.codify)
* Multilingual [models](https://huggingface.co/smallcloudai) under the hood we trained ourselves!
* 20+ languages support: Python, Java, C++, PHP, Javascript, Go and others
* This server allows you to run these models on your hardware, your code doesn't go outside your control
* Use the the 3b version if you have 12 GB of GPU memory for best results
* Real-time code completion
* Privacy settings for projects or even single files

Join our [Discord server](https://www.smallcloud.ai/discord) and follow our
[Twitter](https://twitter.com/refact_ai) to get the latest updates.


## Prerequisities
- Install plugin for your IDE: [JetBrains](https://plugins.jetbrains.com/plugin/20647-refact-ai) or
  [VSCode](https://marketplace.visualstudio.com/items?itemName=smallcloud.codify)
- Sign up or login in to your account
- Large Language Models require a lot of computing resources and memory.
  We recommend using this server with **Nvidia GPU with at least 4Gb VRAM**.
  Another option is to use it with CPU, but it's quite slow and unusable in practice yet.

## Running Server in Docker
The recommended way to run server is a pre-build Docker image.

Install [Docker with NVidia GPU support](https://docs.nvidia.com/datacenter/cloud-native/container-toolkit/install-guide.html#docker).
On Windows you need to install WSL 2 first, [one guide to do this](https://docs.docker.com/desktop/install/windows-install).
Get your **API Key** from refact.ai [account](https://codify.smallcloud.ai/account) page or alternatively from plugin settings.


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
Model weights are saved inside the container. If you remove the container, it will
download the weights again.

Shows messages from the container:
```commandline
docker logs -f
```
</details>

Run docker container with following command:
```commandline
docker run \
       -p 8008:8008 \
       --gpus 0 \
       --name refact_self_hosting \
       --env SERVER_API_TOKEN={API Key} \
       smallcloud/self_hosting
```
Next time you can start it with following command:
```commandline
docker start -i refact_self_hosting
```
After start, container will automatically check for updates and download the chosen model
(see in your [account](https://codify.smallcloud.ai/account)).


## Running Manually

To run server manually, install this repo first (this might install a lot of packages on your computer):
```commandline
pip install git+https://github.com/smallcloudai/code-contrast.git
```
Now you can run server with following command:
```commandline
python -m self_hosting.server \
       --workdir /workdir \
       --token {API Key}
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


## Contributing

We are open for contributions. If you have any ideas and ready to implement this, just:
- make a [fork](https://github.com/smallcloudai/code-contrast/fork)
- make your changes, commit to your fork
- and open a [PR](https://github.com/smallcloudai/code-contrast/fork)
