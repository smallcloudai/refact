SMC_FUNCTIONS_CMD = [
    '/websearch',
    '/vecdb'
]


SMC_FUNCTIONS = [
    # {
    #     'name': 'get_code_examples',
    #     'description': 'searches the vecdb for examples with the code provided. '
    #                    'Called only when user asks information about specific code',
    #     'parameters': {
    #         'type': 'object',
    #         'properties': {
    #             'look_for': {
    #                 'type': 'string',
    #                 'description': 'function name, class name, any other name or code snippet'
    #             }
    #         },
    #         'required': ['look_for']
    #     }
    # },
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
