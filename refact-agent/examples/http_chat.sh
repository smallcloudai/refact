curl http://127.0.0.1:8001/v1/chat -k \
  -H 'Content-Type: application/json' \
  -d '{
  "messages": [
    {"role": "user", "content": "Who is Bill Clinton? What is his favorite programming language?"}
  ],
  "stream": false,
  "temperature": 0.1,
  "max_tokens": 20
}'
