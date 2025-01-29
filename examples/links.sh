#!/usr/bin/env bash

echo
echo Follow up example
curl -X POST http://localhost:8001/v1/links \
    -H "Content-Type: application/json" \
    -d '{
        "meta": {
            "chat_id": "chat-example",
            "chat_remote": false,
            "chat_mode": "EXPLORE"
        },
        "model_name": "gpt-4o-mini",
        "messages": [
            {
                "role": "system",
                "content": "[mode2] You are a refact exploration assistant",
                "tool_calls": null,
                "finish_reason": "",
                "tool_call_id": "",
                "usage": null,
                "subchats": null
            },
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

echo
echo Configuration goto example
curl -X POST http://localhost:8001/v1/links \
    -H "Content-Type: application/json" \
    -d '{
        "meta": {
            "chat_id": "chat-example",
            "chat_remote": false,
            "chat_mode": "AGENT"
        },
        "model_name": "gpt-4o-mini",
        "messages": [
            {
                "role": "system",
                "content": "[mode3] You are a refact agentic assistant",
                "tool_calls": null,
                "finish_reason": "",
                "tool_call_id": "",
                "usage": null,
                "subchats": null
            },
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

echo
echo Summarize project example
curl -X POST http://localhost:8001/v1/links \
    -H "Content-Type: application/json" \
    -d '{
        "meta": {
            "chat_id": "chat-example",
            "chat_remote": false,
            "chat_mode": "NOTOOLS"
        },
        "model_name": "gpt-4o-mini",
        "messages": []
    }'

echo
echo Example of Save and return in configuration chat
curl -X POST http://localhost:8001/v1/links \
    -H "Content-Type: application/json" \
    -d '{
        "meta": {
            "chat_id": "chat-example",
            "chat_remote": false,
            "chat_mode": "CONFIGURE"
        },
        "model_name": "gpt-4o-mini",
        "messages": [
            {
                "role": "system",
                "content": "[mode3config] You are a refact configuration assistant",
                "tool_calls": null,
                "finish_reason": "",
                "tool_call_id": "",
                "usage": null,
                "subchats": null
            },
            {
                "role": "system",
                "content": "[mode3config] You are a refact coding agent working on improving configuration files",
                "tool_calls": null,
                "finish_reason": "",
                "tool_call_id": "",
                "usage": null,
                "subchats": null
            },
            {
                "role": "user",
                "content": "create a new file called \"hello_world.py\"",
                "tool_calls": null,
                "finish_reason": "",
                "tool_call_id": "",
                "usage": null,
                "subchats": null
            },
            {
                "role": "assistant",
                "content": "📍REPLACE_FILE 000 \"/app/hello_world.py\"\n```python\nprint(\"Hello, World!\")\n```",
                "tool_calls": null,
                "finish_reason": "",
                "tool_call_id": "",
                "usage": null,
                "subchats": null
            }
        ]
    }'