SMC_FUNCTIONS = [
    {
        "name": "get_current_weather",
        "description": "Get the current weather in a given location",
        "parameters": {
            "type": "object",
            "properties": {
                "location": {
                    "type": "string",
                    "description": "The city and state, e.g. San Francisco, CA",
                },
                "unit": {"type": "string", "enum": ["celsius", "fahrenheit"]},
            },
            "required": ["location"],
        },
    },
    {
        'name': 'get_code_examples',
        'description': 'searches the vecdb for examples with the code provided. '
                       'Called only when user asks information about specific code',
        'parameters': {
            'type': 'object',
            'properties': {
                'look_for': {
                    'type': 'string',
                    'description': 'function name, class name, any other name or code snippet'
                }
            },
            'required': ['look_for']
        }
    },
    {
        'name': 'web_search',
        'description': 'When user specifies an intention to search for some information (in web), call this function',
        'parameters': {
            'type': 'object',
            'properties': {
                'query': {
                    'type': 'string',
                    'description': 'query to search in google'
                }
            },
            'required': ['query']
        }
    }
]
