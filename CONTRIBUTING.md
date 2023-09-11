# How to Contribute

## Install for Development

Clone this repo and install it for development:

```commandline
git clone https://github.com/smallcloudai/refact
pip install -e refact/
```

To run the whole server, use:

```commandline
python -m self_hosting_machinery.watchdog.docker_watchdog
```

For debugging, it's better to run HTTP server and inference processes separately, for example in
separate terminals.

```commandline
python -m self_hosting_machinery.webgui.webgui
DEBUG=1 python -m self_hosting_machinery.inference.inference_worker --model wizardlm/7b
DEBUG=1 python -m refact_scratchpads_no_gpu.infserver_no_gpu longthink/stable --openai_key sk-XXXYYY
```

That should be enough to get started!

If you plan to make changes, you need your own fork of the project -- clone that instead of
the main repo. Once you have your changes ready, commit them and push them to your fork. After
that you should be abloe to create a pull request for the main repository.


## Adding Toolbox Functions

UPDATE: toolbox is under reconstruction.


## Install Linguist

For fine tuning, files go through a pre filter. Follow instructions in
https://github.com/smallcloudai/linguist
to install it.

If you don't plan to debug fine tuning, you can skip this step.
