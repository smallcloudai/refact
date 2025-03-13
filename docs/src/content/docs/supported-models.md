---
title: Supported Models in Refact
description: Supported Models in Refact
---

## Cloud Version of Refact

### Completion models 
- Refact/1.6B  
- starcoder2/3b

### Chat models
- GPT 3.5
- GPT 4 (Pro plan)

## Self-Hosted Version of Refact

In Refact self-hosted you can select between the following models: 

### Completion models 
<table class="full-table">
<thead>
<tr>
<th>Model Name</th>
<th>Fine-tuning support</th>
</tr>
</thead>
<tbody>
<tr>
<td>Refact/1.6B</td>
<td>✓</td>
</tr>
<tr>
<td>Refact/1.6B/vllm</td>
<td>✓</td>
</tr>
<tr>
<td>starcoder/1b/base</td>
<td>✓</td>
</tr>
<tr>
<td>starcoder/1b/vllm</td>
<td></td>
</tr>
<tr>
<td>starcoder/3b/base</td>
<td>✓</td>
</tr>
<tr>
<td>starcoder/3b/vllm</td>
<td></td>
</tr>
<tr>
<td>starcoder/7b/base</td>
<td>✓</td>
</tr>
<tr>
<td>starcoder/7b/vllm</td>
<td></td>
</tr>
<tr>
<td>starcoder/15b/base</td>
<td></td>
</tr>
<tr>
<td>starcoder/15b/plus</td>
<td></td>
</tr>
<tr>
<td>starcoder2/3b/base</td>
<td>✓</td>
</tr>
<tr>
<td>starcoder2/3b/vllm</td>
<td>✓</td>
</tr>
<tr>
<td>starcoder2/7b/base</td>
<td>✓</td>
</tr>
<tr>
<td>starcoder2/7b/vllm</td>
<td>✓</td>
</tr>
<tr>
<td>starcoder2/15b/base</td>
<td>✓</td>
</tr>
<tr>
<td>deepseek-coder/1.3b/base</td>
<td>✓</td>
</tr>
<tr>
<td>deepseek-coder/1.3b/vllm</td>
<td>✓</td>
</tr>
<tr>
<td>deepseek-coder/5.7b/mqa-base</td>
<td>✓</td>
</tr>
<tr>
<td>deepseek-coder/5.7b/vllm</td>
<td>✓</td>
</tr>
<tr>
<td>codellama/7b</td>
<td>✓</td>
</tr>
<tr>
<td>stable/3b/code</td>
<td></td>
</tr>
<tr>
<td>wizardcoder/15b</td>
<td></td>
</tr>
</tbody>
</table>

### Chat models
<table class="full-table">
<thead>
<tr>
<th>Model Name</th>
</tr>
</thead>
<tbody>
<tr>
<td>starchat/15b/beta</td>
</tr>
<tr>
<td>deepseek-coder/33b/instruct</td>
</tr>
<tr>
<td>deepseek-coder/6.7b/instruct</td>
</tr>
<tr>
<td>deepseek-coder/6.7b/instruct-finetune</td>
</tr>
<tr>
<td>deepseek-coder/6.7b/instruct-finetune/vllm</td>
</tr>
<tr>
<td>wizardlm/7b</td>
</tr>
<tr>
<td>wizardlm/13b</td>
</tr>
<tr>
<td>wizardlm/30b</td>
</tr>
<tr>
<td>llama2/7b</td>
</tr>
<tr>
<td>llama2/13b</td>
</tr>
<tr>
<td>magicoder/6.7b</td>
</tr>
<tr>
<td>mistral/7b/instruct-v0.1</td>
</tr>
<tr>
<td>mixtral/8x7b/instruct-v0.1</td>
</tr>
<tr>
<td>llama3/8b/instruct</td>
</tr>
<tr>
<td>llama3/8b/instruct/vllm</td>
</tr>
</tbody>
</table>

For an up-to-date list of models, see the [Known Models file on GitHub](https://github.com/smallcloudai/refact-lsp/blob/main/src/known_models.rs).

## Integrations

Refact.ai offers **OpenAI** and **Anthropic API** integrations.

To enable these integrations, navigate to the **Model Hosting** page activate the **OpenAI** and/or **Anthropic** integrations by pressing the switch button in the **3rd Party APIs** section.

![3rd Party APIs Secction](../../assets/3-party-apis.png)

Press **API Keys tab** link, you will be redirected to the integrations page. Alternatively, you can access the integrations page by clicking on the **Settings** dropdown menu in the header and selecting **Credentials**.

![Model Hosting Page with Dropdown Expanded](../../assets/api-keys-link.png)

In the **Credentials** page, you can specify your **OpenAI** and/or **Anthropic** API keys.

:::note
Make sure the switch button is enabled for each API you want to use. Even if you specify the API key, it will not be used until the switch button is enabled.
:::