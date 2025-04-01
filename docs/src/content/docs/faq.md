---
title: FAQ
description: Frequently Asked Question(s)
---

### What programming languages do you support?

We support multiple code LLMs, each of them has been trained on different programming languages. 

Our own model Refact 1.6B has been trained and works best for the following languages (alphabetical order): Bash, C#, C++, D, Golang, Java, JavaScript, Julia, Lua, Perl, PHP, Python, R, Racket, Ruby, Rust, Scala, Swift, TypeScript.
Even if the model has not been specifically trained on a particular coding language, it can still make useful suggestions for code in that language. 

### What models are under the hood? 

We use a combination of our own models and 3rd party models for different functions. 

Our own model is Refact 1.6B code LLM. It's State-of-the-art for the size and In addition to regular prompting, this model can infill code in the middle and produce changes to the code by following instructions. Check it out https://huggingface.co/smallcloudai/Refact-1_6B-fim 

For chat we use models from the GPT family, you have the option to opt-out of them.  
In the self-hosted version we also have StarCoder, Code Llama and WizardCoder models. 

For a full list of our supported models and their functionality, check our [docs](https://docs.refact.ai/supported-models/). 

### Do you plan to support more IDEs? 

Yes! We already support VS Code and JetBrains. We have plans to support even more IDEs. If you want to contribute to our new plugins, please reach us out in Discord.

### Do you have a self-hosted option? 
Yes. Refact has a free self-hosted version that you can check here. 

### Is it possible to fine-tune Refact to the company codebase? 
Yes. Fine-tuning is currently supported in our free self-hosted and Enterprise plans. 

### Can I buy Refact license for my company?

Sure! We currently have an Enterprise self-hosted plan and we plan to add team cloud plan soon. If you're interested in purchasing a license for your company, please [contact us](https://refact.ai/contact). 

### How can I contribute? 
We welcome contributions! If you're interested in contributing, please check our [GitHub](https://github.com/smallcloudai/refact/). 
