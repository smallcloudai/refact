let textarea, saving_toolbox = false;
function get_toolxbox_file() {
    fetch("/toolbox.yaml")
    .then(function(response) {
        return response.text();
    })
    .then(function(data) {
        textarea.innerHTML = data;
        delete textarea.dataset['highlighted'];
        hljs.highlightElement(textarea);
    });
}

function save_toolxbox_file() {
    if(saving_toolbox) return;
    saving_toolbox = true;
    const data = textarea.textContent
    fetch('/tab-toolbox-upload', {
        method: 'POST',
        headers: {
            'Content-Type': 'text/yaml'
        },
        body: data
    })
    .then(response => {
        get_toolxbox_file();
    })
    .catch(error => {
        console.log(error.message);
    });
}

export async function init(general_error) {
    let req = await fetch('/tab-toolbox.html');
    document.querySelector('#toolbox').innerHTML = await req.text();
    textarea = document.querySelector('.language-yaml');
    const editor_save_button = document.querySelector('.settings-tab-toolbox-submit');
    editor_save_button.addEventListener('click', save_toolxbox_file);
}

export function tab_switched_here() {
    get_toolxbox_file();
}

export function tab_switched_away() {
}

export function tab_update_each_couple_of_seconds() {
}
