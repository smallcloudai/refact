<p align="center">
  <img width="300" alt="Refact" src="refact-logo.svg"/>
</p>

---

[![Discord](https://img.shields.io/discord/1037660742440194089?logo=discord&label=Discord&link=https%3A%2F%2Fsmallcloud.ai%2Fdiscord)](https://smallcloud.ai/discord)
[![Twitter Follow](https://img.shields.io/twitter/follow/refact_ai)](https://twitter.com/intent/follow?screen_name=refact_ai)
![License](https://img.shields.io/github/license/smallcloudai/refact?cacheSeconds=1000)
[![Visual Studio](https://img.shields.io/visual-studio-marketplace/d/smallcloud.codify?label=VS%20Code)](https://marketplace.visualstudio.com/items?itemName=smallcloud.codify)
[![JetBrains](https://img.shields.io/jetbrains/plugin/d/com.smallcloud.codify?label=JetBrains)](https://plugins.jetbrains.com/plugin/20647-codify)

Refact is an open-source Copilot alternative available as a self-hosted or cloud option.
- [x] Autocompletion powered by best-in-class open-source code models 
- [x] Context-aware chat on a current file
- [x] Refactor, explain, analyse, optimise code, and fix bug functions 
- [x] Fine-tuning on codebase (Beta, self-hosted only) [Docs](https://refact.ai/docs/fine-tuning/)
- [ ] Context-aware chat on entire codebase 
      
![Image Description](./almost-all-features-05x-dark.jpeg)

## Getting Started  

1. Download Refact for [VS Code](https://marketplace.visualstudio.com/items?itemName=smallcloud.codify) or [JetBrains](https://plugins.jetbrains.com/plugin/20647-refact-ai)
2. For the cloud version, create your account at [https://refact.ai/](https://refact.ai/) and start immediately. For the self-hosted version, follow the instructions [here](https://refact.ai/docs/self-hosting/).  

## How Refact works 

Under the hood, Refact uses the best open-source models.

At the moment, you can choose between the following models:

| Model                                                                                | Completion | Chat      | AI Toolbox | Fine-tuning | 
| ------------------------------------------------------------------------------------ | ---------- | --------- | ---------- | ----------| 
| [CONTRASTcode/medium/multi](https://huggingface.co/smallcloudai/codify_medium_multi) |    +    |           |           |            |  
| [CONTRASTcode/3b/multi](https://huggingface.co/smallcloudai/codify_3b_multi)         |    +    |           |           |        +    |    
| [starcoder/15b/base](https://huggingface.co/smallcloudai/starcoder_15b_4bit)         |   +     |          |           |           |   
| [starcoder/15b/base8bit](https://huggingface.co/smallcloudai/starcoder_15b_8bit)     |    +    |          |           |           |  
| starchat/15b/beta                                                                     |        |         + |           |          | 
| wizardcoder/15b                                                                       |     +   |          |           |           | 
| wizardlm/7b |        |         + |           |         |
| wizardlm/13b  |        |         + |           |          |
| llama2/7b    |        |         + |          |         |
| llama2/13b   |        |         + |           |           |

## Usage
Refact is free to use for individuals and small teams under BSD-3-Clause license. If you wish to use Refact for Enterprise, please [contact us](https://refact.ai/contact/). 

## FAQ

Q: Can I run a model on CPU?

A: it doesn't run on CPU yet, but it's certainly possible to implement this.

Q: Sharding is disabled, why?

A: It's not ready yet, but it's coming soon.

## Community & Support

- Contributing [CONTRIBUTING.md](CONTRIBUTING.md)
- [GitHub issues](https://github.com/smallcloudai/refact/issues) for bugs and errors 
- [Community forum](https://github.com/smallcloudai/refact/discussions) for community support and discussions
- [Discord](https://www.smallcloud.ai/discord) for chatting with community members
- [Twitter](https://twitter.com/refact_ai) for product news and updates 

