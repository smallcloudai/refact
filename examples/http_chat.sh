curl http://127.0.0.1:8001/v1/chat -k \
  -H 'Content-Type: application/json' \
  -d '{
  "messages": [
    {"role": "user", "content": "@local-notes-to-self\nWho is Bill Clinton? What is his favorite programming language?"}
  ],
  "stream": true,
  "parameters": {
    "temperature": 0.1,
    "max_new_tokens": 1000
  }
}'

# Other possible parameters:
# "model": "meta-llama/Llama-2-70b-chat-hf",
