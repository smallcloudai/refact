# Follow up example
curl -X POST http://localhost:8001/v1/links \
    -H "Content-Type: application/json" \
    -d '{
        "chat_id": "chat-example",
        "model_name": "gpt-4o-mini",
        "messages": [
            {
                "role": "user",
                "content": "List all containers",
                "tool_calls": null,
                "finish_reason": "",
                "tool_call_id": "",
                "usage": null,
                "subchats": null
            },
            {
                "role": "assistant",
                "content": "Docker tool not found, you can use cli and run docker ps",
                "tool_calls": null,
                "finish_reason": "",
                "tool_call_id": "",
                "usage": null,
                "subchats": null
            }
        ]
    }'

# Configuration goto example
curl -X POST http://localhost:8001/v1/links \
    -H "Content-Type: application/json" \
    -d '{
        "chat_id": "chat-example",
        "model_name": "gpt-4o-mini",
        "messages": [
            {
                "role": "user",
                "content": "use psql tool to list all tables",
                "tool_calls": null,
                "finish_reason": "",
                "tool_call_id": "",
                "usage": null,
                "subchats": null
            },
            {
                "role": "assistant",
                "content": null,
                "tool_calls": [
                    {
                        "id": "call_BGLjpFFQJbD00euir9rQ8V7w",
                        "function": {
                            "arguments": "{\"query\":\"\\\\dt;\"}",
                            "name": "postgres"
                        },
                        "type": "function"
                    }
                ],
                "finish_reason": "tool_calls",
                "tool_call_id": "",
                "usage": null,
                "subchats": null
            },
            {
                "role": "tool",
                "content": "🧩 for configuration go to SETTINGS:postgres, psql failed:\nNo such file or directory (os error 2)",
                "tool_calls": null,
                "finish_reason": "",
                "tool_call_id": "call_BGLjpFFQJbD00euir9rQ8V7w",
                "usage": null,
                "subchats": null
            }
        ]
    }'

# Summarize project example
curl -X POST http://localhost:8001/v1/links \
    -H "Content-Type: application/json" \
    -d '{
        "chat_id": "chat-example",
        "model_name": "gpt-4o-mini",
        "messages": []
    }'