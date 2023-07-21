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

Are you missing a function in the toolbox? It's easy to implement it yourself!

It's even possible without a GPU, clone this repo and install it like this:

```
SETUP_PACKAGE=refact_scratchpads_no_gpu pip install -e refact/
```

In this folder `refact_scratchpads_no_gpu/gpt_toolbox/toolbox_functions` there are some
functions implemented using OpenAI API. There you can add a new one by analogy, or even
make an existing function better.

Add your new function to `infserver_no_gpu.py` and `modelcap_records.py`.

To test your function, run `infserver_no_gpu` as in the previous section.


## Simplifying Toolbox (WORK IN PROGRESS)

1. Toolbox for models with GPU https://github.com/smallcloudai/refact/pull/33

2. Simplify functions list, so you don't have to touch `infserver_no_gpu.py` and `modelcap_records` (no PR yet)


## Install Linguist

For fine tuning, files go through a pre filter. Follow instructions in
https://github.com/smallcloudai/linguist
to install it.

If you don't plan to debug fine tuning, you can skip this step.
