import json
j = {"cloud_name":"Refact Self-Hosted","endpoint_template":"v1/completions","endpoint_chat_passthrough":"v1/chat/completions","endpoint_style":"openai","telemetry_basic_dest":"/stats/telemetry-basic","telemetry_corrected_snippets_dest":"/stats/telemetry-snippets","running_models":["mistral/7b/instruct-v0.1","deepseek-coder/6.7b/instruct","Refact/1.6B/vllm","gpt-3.5-turbo","gpt-4"],"code_completion_default_model":"Refact/1.6B/vllm","code_chat_default_model":"mistral/7b/instruct-v0.1","tokenizer_path_template":"https://huggingface.co/$MODEL/resolve/main/tokenizer.json","tokenizer_rewrite_path":{"mistral/7b/instruct-v0.1":"TheBloke/Mistral-7B-Instruct-v0.1-GPTQ","deepseek-coder/6.7b/instruct":"TheBloke/deepseek-coder-6.7B-instruct-GPTQ","Refact/1.6B/vllm":"smallcloudai/Refact-1_6B-fim"}}

print(json.dumps(j, indent=4))

