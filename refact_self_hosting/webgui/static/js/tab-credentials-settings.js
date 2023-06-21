let chat_gpt_api_key_focused = false;
function get_ssh_keys() {
    fetch("/tab-settings-get-all-ssh-keys")
        .then(function(response) {
            return response.json();
        })
        .then(function(data) {
            console.log('get-all-ssh-keys',data);
            render_keys(data);
        });
}

function render_keys(data) {
    let key_list = document.querySelector('.settings-all-keys');
    key_list.innerHTML = '';
    data.forEach(function (key) {
        let key_wrap = document.createElement('div');
        key_wrap.classList.add('tab-settings-ssh-key-item');
        const date = new Date(key.created_ts * 1000);
        const date_string = date.toLocaleString();
        key_wrap.innerHTML = `
            <div class="tab-settings-ssh-key-content">
                <h6 class="tab-settings-ssh-key-name"><i class="bi bi-key"></i>${key.name}</h6>
                <div class="tab-settings-ssh-key-fingerprint">${key.fingerprint}</div>
                <div class="tab-settings-ssh-key-created">${date_string}</div>
            </div>
            <button data-key="${key.name}" class="tab-settings-ssh-key-delete btn btn-danger btn-sm"><i class="bi bi-trash3-fill"></i></button>
        `;
        key_list.appendChild(key_wrap);
    });
}

function delete_ssh_key(event) {
    if (event.target.classList.contains("bi-trash3-fill")) {
        console.log()
        event.target.parentNode.disabled = true;
        const key_name = event.target.parentNode.dataset.key;
        fetch('/tab-settings-delete-ssh-key', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify({'name':key_name})
        }).then(response => {
            if (!response.ok) {
                return response.json()
                .then(error => {
                    throw new Error(error.message);
                });
            }
            return response.json();
        })
        .then(data => {
            console.log('delete ssh key', data);
        })
        .catch(error => {
            document.querySelector('#status-ssh').innerHTML = error.message;
        });
    }
}

export function init() {
    let key_list = document.querySelector('.settings-all-keys');
    key_list.addEventListener("click", delete_ssh_key);
    const ssh_modal = document.getElementById('settings-tab-ssh-modal');
    ssh_modal.addEventListener('show.bs.modal', function () {
        ssh_modal.querySelector('#tab-settings-key-title-input').value = '';
        ssh_modal.querySelector('.tab-settings-ssh-keywrap').classList.add('d-none');
        ssh_modal.querySelector('.tab-settings-ssh code').innerHTML = '';
        ssh_modal.querySelector('#status-ssh').innerHTML = '';
        let ssh_button = document.querySelector('.settings-tab-ssh-submit');
        ssh_button.style.display = 'inline-block';
    });
    const new_ssh_key_submit = document.querySelector('.settings-tab-ssh-submit');
    new_ssh_key_submit.addEventListener('click', function () {
        let key_name = 'default';
        if(document.querySelector('#tab-settings-key-title-input').value !== '') {
            key_name = document.querySelector('#tab-settings-key-title-input').value;
        }

        fetch('/tab-settings-create-ssh-key', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify({'name':key_name})
        })
        .then(response => {
            if (!response.ok) {
                return response.json()
                .then(error => {
                    throw new Error(error.message);
                });
            }
            return response.json();
        })
        .then(data => {
            console.log('new ssh key created', data);
            let ssh_wrap = document.querySelector('.tab-settings-ssh-keywrap');
            ssh_wrap.classList.remove('d-none');
            let ssh_field = document.querySelector('.tab-settings-ssh code');
            ssh_field.innerHTML = data.public_key;
            let ssh_button = document.querySelector('.settings-tab-ssh-submit');
            ssh_button.style.display = 'none';
        })
        .catch(error => {
            document.querySelector('#status-ssh').innerHTML = error.message;
        });
    });

    const enable_chat_gpt_switch = document.getElementById('enable_chat_gpt');
    enable_chat_gpt_switch.addEventListener('change', function () {
        set_enable_chat_gpt(this.checked)
    })
    const chat_gpt_apikey_button = document.getElementById('integrations-save');
    chat_gpt_apikey_button.addEventListener('click', function () {
        const chat_gpt_apikey_textedit = document.getElementById('chat_gpt_key');
        set_chat_gpt_api_key(chat_gpt_apikey_textedit.value)
    })
    const chat_gpt_api_input = document.getElementById('chat_gpt_key');
    chat_gpt_api_input.addEventListener('focus', function () {
        chat_gpt_api_key_focused = true;
    })
}

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
        chat_gpt_api_key_focused = false;
    });
}

export function update_integrations() {
    fetch("/tab-api-key-settings-get-chat-gpt-info")
        .then(function(response) {
            return response.json();
        })
        .then(function(data) {
            console.log('get-chat-gpt-info',data);
            const enable_chat_gpt_switch = document.getElementById('enable_chat_gpt');
            if(data.api_key === '') {
                enable_chat_gpt_switch.checked = false;
                enable_chat_gpt_switch.disabled = true;
                if(data.is_enabled) {
                    set_enable_chat_gpt(false);
                }
                return;
            }
            enable_chat_gpt_switch.disabled = false;
            enable_chat_gpt_switch.checked = data['is_enabled']
            const chat_gpt_apikey_textedit = document.getElementById('chat_gpt_key');
            if(!chat_gpt_api_key_focused) {
                chat_gpt_apikey_textedit.value = data['api_key']
            }
        });
}


export function tab_switched_here() {
    get_ssh_keys();
    update_integrations();
    // render_time_dropdown();
}