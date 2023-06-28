<div align="center">

# <font color="red">[{</font> Refact.ai Inference Server

</div>

This is a self-hosted server for the [refact.ai](https://www.refact.ai) coding assistant.

With Refact you can run high-quality AI code completions on-premise and use a number of
functions for code transformation and ask questions in the chat.

This server allows you to run AI coding models on your hardware, your code doesn't go outside your control.

At the moment, you can choose between following models:

| Model                                                                                | GPU (VRAM) | CPU (RAM) | Completion | AI Toolbox | Chat | Languages supported                                |
| ------------------------------------------------------------------------------------ | ---------- | --------- | ---------- | ---------- | ---- | -------------------------------------------------- |
| [CONTRASTcode/medium/multi](https://huggingface.co/smallcloudai/codify_medium_multi) |        3Gb |       3Gb |          + |            |      | [20+ Programming Languages](https://refact.ai/faq) |
| [CONTRASTcode/3b/multi](https://huggingface.co/smallcloudai/codify_3b_multi)         |        8Gb |      12Gb |          + |            |      | [20+ Programming Languages](https://refact.ai/faq) |
| [starcoder/15b/base4bit](https://huggingface.co/smallcloudai/starcoder_15b_4bit)     |       16Gb |         - |          + |          + |    + | [80+ Programming languages](https://huggingface.co/blog/starcoder) |
| [starcoder/15b/base8bit](https://huggingface.co/smallcloudai/starcoder_15b_8bit)     |       32Gb |         - |          + |          + |    + | [80+ Programming languages](https://huggingface.co/blog/starcoder) |

Refact is currently available as a plugin for [JetBrains](https://plugins.jetbrains.com/plugin/20647-refact-ai)
products and [VS Code IDE](https://marketplace.visualstudio.com/items?itemName=smallcloud.codify).

NEW: you can fine-tune the model on your own hardware!


## Known limitations

- For best results on smaller GPUs we recommend using CONTRASTcode models as the StarCoder models can be quite slow


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
[GitHub issues](https://github.com/smallcloudai/refact-self-hosting/issues).


and set up this inference url in plugin:

```commandline
https://inference.smallcloud.local:8008
```
</details>


## Fine Tuning

*Why?*  Code models are trained on a vast amount of code from the internet, which may not perfectly
align with your specific codebase, APIs, objects, or coding style.
By fine-tuning the model, you can make it more familiar with your codebase and coding patterns.
This allows the model to better understand your specific needs and provide more relevant and
accurate code suggestions. Fine-tuning essentially helps the model memorize the patterns and
structures commonly found in your code, resulting in improved suggestions tailored to your
coding style and requirements.

*Which Files to Feed?*  It's a good idea to give the model the current code of your projects,
because it's likely any new code in the same project will be similar -- that's what makes
suggestions relevant and useful. However, it's NOT a good idea feed 3rd party libraries that
you use, as the model may learn to generate code similar to the internals of those libraries.

*GUI*  Use `Sources` and `Finetune` tabs in the web UI to upload files (.zip, .gz, .bz2 archive, or
a link to your git repository) and run the fine-tune process. After the fine-tuning process
finishes (which should take several hours), you can dynamically turn it on and off and observe
the difference it makes for code suggestions.

There's a catch: both VS Code and JB plugins cache the responses. To force the model to produce
a new suggestion (rather than immediately responding with a cached one), you can change the text
a few lines above, for example, a comment. This will make the code look different. Alternatively,
you can use the Manual Suggestion Trigger (a key combination), which always produces a new suggestion.



## Community & Support

Join our
[Discord server](https://www.smallcloud.ai/discord) and follow our
[Twitter](https://twitter.com/refact_ai) to get the latest updates.



## Contributing

We are open for contributions. If you have any ideas and ready to implement this, just:
- make a [fork](https://github.com/smallcloudai/refact-self-hosting/fork)
- make your changes, commit to your fork
- and open a PR
