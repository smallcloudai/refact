
function fetch_and_set_files_loaded_cnt() {
    fetch("/tab-vecdb-files-stats")
        .then(function(response) {
            if (response.ok) {
                return response.json();
            }
            return {'files_cnt': 'error', 'chunks_cnt': 'error'}
        })
        .then(function(data) {
            document.querySelector('#vecdb-files-loaded-cnt').innerHTML = data['files_cnt'];
            document.querySelector('#vecdb-chunks-loaded-cnt').innerHTML = data['chunks_cnt'];
        });
}



function on_select_model_modal_ok_click(event) {
    let select_model_div = document.getElementById('vecdb-select-model');
    let selected_option = select_model_div.getAttribute('data-selected-temp');
    // select_model_div.setAttribute('data-selected', selected_option);
    select_model_div.setAttribute('data-selected-temp', "");
    select_model_div.setAttribute('data-ok', "true");
    select_model_div.setAttribute('disabled', "");
    console.log("ok click")

    fetch('/tab-vecdb-update-provider', {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify({
            'provider': selected_option
        })
    });
}

function on_select_model_modal_hide(event) {
    let select_model_div = document.getElementById('vecdb-select-model');
    if (select_model_div.getAttribute('data-ok') === "true") {
        select_model_div.removeAttribute('data-ok');
        console.log('on_modal_hide, but data-ok');
        return;
    }
    console.log('on_modal_hide', select_model_div.getAttribute('data-ok'));
    let prev_option = select_model_div.getAttribute('data-selected');
    document.querySelector(`[value="${prev_option}"]`).selected = true;
}

export async function init() {
    let req = await fetch('/tab-context.html');
    document.querySelector('#context').innerHTML = await req.text();

    document.getElementById('vecdb-select-model-modal-ok').addEventListener(
        'click', on_select_model_modal_ok_click
    )

    document.getElementById('vecdb-select-model-modal').addEventListener(
        'hidden.bs.modal', on_select_model_modal_hide
    )

    await render_vecdb_status();
    fetch_and_set_files_loaded_cnt();
}


export async function tab_switched_here() {
    await render_vecdb_status();
    fetch_and_set_files_loaded_cnt();
}


export async function tab_switched_away() {

}



async function vecdb_render_select_model(available_providers, provider) {
    let select_model_div = document.getElementById('vecdb-select-model');
    if (
        select_model_div.getAttribute('data-selected') === provider &&
        select_model_div.getAttribute('data-options') === JSON.stringify(available_providers)
    ) {
        return;
    }
    console.log('fetching models...')
    select_model_div.innerHTML = '';
    for (let p of available_providers) {
        let option = document.createElement('option');
        option.value = p;
        option.innerHTML = p;
        option.classList.add('vecdb-select-model-option');
        if (p === provider) {
            option.selected = true;
        }
        select_model_div.appendChild(option);
    }

    select_model_div.setAttribute('data-selected', provider);
    select_model_div.setAttribute('data-options', JSON.stringify(available_providers));

    select_model_div.addEventListener('change', (event) => {
        select_model_div.setAttribute('data-selected-temp', event.target.value);
        console.log('selected', event.target.value);
        bootstrap.Modal.getOrCreateInstance(document.getElementById('vecdb-select-model-modal')).show();
    });
}


async function render_vecdb_status() {
    function fetch_vecdb_status() {
        return fetch('/tab-vecdb-status').then(
            function (response) {
                if (response.ok) {
                    return response.json();
                } else {
                    return {}
                }
            }
        )
    }

    function set_health_status(vecdb_status) {
        let health_display_text = vecdb_status['status'];
        if (health_display_text === 'ok') {
            health_display_text = 'healthy ❤️';
        }
        document.querySelector('#vecdb-health').innerHTML = health_display_text;

    }

    function check_ongoing(vecdb_status) {
        let select_model_div = document.getElementById('vecdb-select-model');
        let ongoing = vecdb_status['ongoing'];
        if (ongoing && 'indexing' in ongoing) {
            const indexing_status = ongoing['indexing']['status'];
            let indexing_status_span = document.getElementById('vecdb-indexing-status')
            indexing_status_span.innerHTML = indexing_status;
            if (indexing_status !== 'done') {
                select_model_div.setAttribute('disabled', "");
            } else {
                let prev_state = indexing_status_span.getAttribute('data-prev-state');
                if (prev_state && prev_state !== 'done') {
                    select_model_div.removeAttribute('disabled');
                    fetch_vecdb_status()
                }
            }
            indexing_status_span.setAttribute('data-prev-state', indexing_status);
        } else {
            document.getElementById('vecdb-indexing-status').innerHTML = 'Not scheduled';
            select_model_div.removeAttribute('disabled');
        }
    }


    fetch_vecdb_status().then(async (vecdb_status) => {
        check_ongoing(vecdb_status);
        set_health_status(vecdb_status);
        await vecdb_render_select_model(
            vecdb_status['available_providers'], vecdb_status['provider']
        )
    });

}


export async function tab_update_each_couple_of_seconds() {
    await render_vecdb_status()
}