---
title: Enterprise Refact Edition - Model Hosting
description: What Enterprise Refact is and how it works.
---

Refact Enterprise Refact is a version that is optimized for enterprise use cases. It allows you to use all of the models available in Refact.ai Self-hosted and also supports vLLM models.

### Enabling vLLM

With the enterprise version of Refact, you can use an inference engine that uses `PagedAttention` from the vLLM library. It works faster and supports continuous batching, which means it can start work on new inference tasks, while continuing to serve other clients at the same time.

To enable vLLM select one the available vLLM models in the **Model Hosting** page. The full list of available models can be found on the [Supported Models page](https://docs.refact.ai/supported-models/).

:::note
vLLM models are suitable for a fast inference. The limitation with the vLLM models is that they **don't support sharding**.
:::