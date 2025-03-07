import { general_error } from './error.js';
// let chat_gpt_api_key_focused = false;
let show_toast = false;

function get_ssh_keys() {
    fetch("/tab-settings-get-all-ssh-keys")
        .then(function(response) {
            return response.json();
        })
        .then(function(data) {
            console.log('get-all-ssh-keys',data);
            render_keys(data);
        })
       .catch(function(error) {
            console.log('tab-settings-get-all-ssh-keys',error);
            general_error(error);
        });
}

function render_keys(data) {
    let key_list = document.querySelector('.settings-all-keys');
    if(data.length > 0) {
        key_list.innerHTML = '';
    }
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
            get_ssh_keys();
        })
        .catch(error => {
            document.querySelector('#status-ssh').innerHTML = error.message;
        });
    }
}

export async function init(general_error) {
    let req = await fetch('/tab-settings.html');
    document.querySelector('#settings').innerHTML = await req.text();
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
            let ssh_hide_modal_button = document.querySelector('.settings-tab-ssh-close');
            ssh_hide_modal_button.classList.remove('d-none');
            get_ssh_keys();
        })
        .catch(error => {
            document.querySelector('#status-ssh').innerHTML = error.message;
        });
    });

    const model_links = document.querySelectorAll('.settings-tab-model-link');
    model_links.forEach(function (link) {
        link.addEventListener('click', function (event) {
            document.querySelector(`[data-tab=${link.getAttribute('data-tab')}]`).click();
        });
    });
}

function mask_integrations_input(el) {
    function mask_string(string) {
        if (string.length > 6 ) {
            return string.substring(0, 6) + '*'.repeat(string.length - 6);
        } else {
            return '*'.repeat(string.length)
        }
    }
    if (!el.getAttribute('data-masked')) {
        el.setAttribute('data-value', el.value);
        el.value = mask_string(el.getAttribute('data-value'));
        el.setAttribute('data-masked', 'true')
    }
}

function unmask_integrations_input(el) {
    if (el.getAttribute('data-masked') === 'true') {
        el.value = el.getAttribute('data-value');
        el.removeAttribute('data-masked');
    }
}


function throw_int_saved_success_toast(msg) {
    let int_saved_success_toast_div = document.querySelector('.int-saved-success-toast');
    const success_toast = bootstrap.Toast.getOrCreateInstance(int_saved_success_toast_div);
    if (!show_toast) {
        console.log('not show toast')
        show_toast = true;
        document.querySelector('.int-saved-success-toast .toast-body').innerHTML = msg;
        success_toast.show();
        setTimeout(function () {
            success_toast.hide();
            show_toast = false;
        }, 2000);
    }
}

function save_integration_api_keys() {
    const openai_api_key = document.getElementById('openai_api_key');
    const anthropic_api_key = document.getElementById('anthropic_api_key');
    const groq_api_key = document.getElementById('groq_api_key');
    const cerebras_api_key = document.getElementById('cerebras_api_key');
    const gemini_api_key = document.getElementById("gemini_api_key");
    const xai_api_key = document.getElementById('xai_api_key');
    const deepseek_api_key = document.getElementById('deepseek_api_key');

    const huggingface_api_key = document.getElementById('huggingface_api_key');
    fetch("/tab-settings-integrations-save", {
        method: "POST",
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify({
            openai_api_key: openai_api_key.getAttribute('data-value'),
            anthropic_api_key: anthropic_api_key.getAttribute('data-value'),
            groq_api_key: groq_api_key.getAttribute('data-value'),
            cerebras_api_key: cerebras_api_key.getAttribute('data-value'),
            gemini_api_key: gemini_api_key.getAttribute("data-value"),
            xai_api_key: xai_api_key.getAttribute('data-value'),
            deepseek_api_key: deepseek_api_key.getAttribute('data-value'),

            huggingface_api_key: huggingface_api_key.getAttribute('data-value'),
        })
    })
    .then(function(response) {
        console.log(response);
        throw_int_saved_success_toast('API Key saved')
        openai_api_key.setAttribute('data-saved-value', openai_api_key.getAttribute('data-value'))
        anthropic_api_key.setAttribute('data-saved-value', anthropic_api_key.getAttribute('data-value'))
        groq_api_key.setAttribute('data-saved-value', groq_api_key.getAttribute('data-value'))
        cerebras_api_key.setAttribute('data-saved-value', cerebras_api_key.getAttribute('data-value'))
        gemini_api_key.setAttribute('data-saved-value', gemini_api_key.getAttribute('data-value'))
        xai_api_key.setAttribute('data-saved-value', xai_api_key.getAttribute('data-value'))
        deepseek_api_key.setAttribute('data-saved-value', deepseek_api_key.getAttribute('data-value'))

        huggingface_api_key.setAttribute('data-saved-value', huggingface_api_key.getAttribute('data-value'))
    });
}


function integrations_input_init(element, data) {
    if (data) {
        element.value = data
    }
    mask_integrations_input(element);
    element.setAttribute('data-saved-value', element.getAttribute('data-value'))
    element.addEventListener(
        'focus', () => unmask_integrations_input(element)
    )
    element.addEventListener(
        'blur', () => {
            mask_integrations_input(element)
            if (element.getAttribute('data-value') !== element.getAttribute('data-saved-value')) {
                save_integration_api_keys();
            }
        }
    )
}


export function tab_settings_integrations_get() {
    fetch("/tab-settings-integrations-get")
        .then(function(response) {
            return response.json();
        })
        .then(function(data) {
            integrations_input_init(document.getElementById('openai_api_key'), data['openai_api_key']);
            integrations_input_init(document.getElementById('anthropic_api_key'), data['anthropic_api_key']);
            integrations_input_init(document.getElementById('groq_api_key'), data['groq_api_key']);
            integrations_input_init(document.getElementById('cerebras_api_key'), data['cerebras_api_key']);
            integrations_input_init(document.getElementById('gemini_api_key'), data['gemini_api_key']);
            integrations_input_init(document.getElementById('xai_api_key'), data['xai_api_key']);
            integrations_input_init(document.getElementById('deepseek_api_key'), data['deepseek_api_key']);

            integrations_input_init(document.getElementById('huggingface_api_key'), data['huggingface_api_key']);
        });
}


export function tab_switched_here() {
    get_ssh_keys();
    tab_settings_integrations_get();
}

export function tab_switched_away() {
}

export function tab_update_each_couple_of_seconds() {
}
