function set_enable_chat_gpt(is_enabled) {
    fetch("/tab-api-key-settings-set-enabled-chat-gpt", {
        method: "POST",
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify({
            is_enabled: is_enabled,
        })
    })
    .then(function(response) {
        console.log(response);
    });
}

function set_chat_gpt_api_key(api_key) {
    fetch("/tab-api-key-settings-set-chat-gpt-api-key", {
        method: "POST",
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify({
            api_key: api_key,
        })
    })
    .then(function(response) {
        console.log(response);
    });
}

export function init() {
    const enable_chat_gpt_switch = document.getElementById('enable_chat_gpt');
    enable_chat_gpt_switch.addEventListener('change', function () {
        set_enable_chat_gpt(this.checked)
    })
    const chat_gpt_apikey_textedit = document.getElementById('chat_gpt_key');
    chat_gpt_apikey_textedit.addEventListener('focusout', function () {
        set_chat_gpt_api_key(this.value)
    })
}

function get_info() {
    fetch("/tab-api-key-settings-get-chat-gpt-info")
        .then(function(response) {
            return response.json();
        })
        .then(function(data) {
            const enable_chat_gpt_switch = document.getElementById('enable_chat_gpt');
            enable_chat_gpt_switch.checked = data['is_enabled']
            const chat_gpt_apikey_textedit = document.getElementById('chat_gpt_key');
            chat_gpt_apikey_textedit.text = data['api_key']
        });
}

export function tab_switched_here() {
    get_info()
}

