curl http://127.0.0.1:8001/v1/vdb-search -k \
  -H 'Content-Type: application/json' \
  -d '{
  "query": "Hello world",
  "top_n": 3
}'

