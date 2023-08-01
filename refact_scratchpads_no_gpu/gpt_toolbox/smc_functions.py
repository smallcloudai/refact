SMC_FUNCTIONS_CMD = [
    '/vecdb'
]


SMC_FUNCTIONS = [
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
