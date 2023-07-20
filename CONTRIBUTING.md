Clone this repo and install it for development:

git clone https://github.com/smallcloudai/refact
pip install -e refact/

To run the whole server, use:

python -m self_hosting_machinery.watchdog.docker_watchdog

For debugging, it's better to run HTTP server and inference processes separately, for example in separate terminals.

python -m self_hosting_machinery.webgui.webgui
DEBUG=1 python -m self_hosting_machinery.inference.inference_worker --model wizardlm/7b
DEBUG=1 python -m refact_scratchpads_no_gpu.infserver_no_gpu longthink/stable --openai_key sk-XXXYYY

Adding Toolbox Functions

Are you missing a function in the toolbox? It's easy to implement it yourself!

It's even possible without a GPU, clone this repo and install it like this:

SETUP_PACKAGE=refact_scratchpads_no_gpu pip install -e refact/

In this folder refact_scratchpads_no_gpu/gpt_toolbox/toolbox_functions there are some functions implemented using OpenAI API. There you can add a new one by analogy, or even make an existing function better.

To test your function, run infserver_no_gpu as in the previous section.
