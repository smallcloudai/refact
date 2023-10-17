function on_select_model_modal_ok_click(event) {
    let select_model_div = document.getElementById('vecdb-select-model');
    let selected_option = select_model_div.getAttribute('data-selected-temp');
    select_model_div.setAttribute('data-selected-temp', "");
    select_model_div.setAttribute('data-ok', "true");
    select_model_div.setAttribute('disabled', "");

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

    let select_model_div = document.getElementById('vecdb-select-model');
    select_model_div.addEventListener('change', (event) => {
        select_model_div.setAttribute('data-selected-temp', event.target.value);
        bootstrap.Modal.getOrCreateInstance(document.getElementById('vecdb-select-model-modal')).show();
    });
}


export async function tab_switched_here() {
    await render_vecdb_status();
}


export async function tab_switched_away() {

}


async function render_vecdb_status() {
    async function vecdb_render_select_model(available_providers, provider) {
        let select_model_div = document.getElementById('vecdb-select-model');
        if (select_model_div.getAttribute('data-selected') === provider || !provider || !available_providers) {
            return;
        }
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
        // select_model_div.setAttribute('data-options', JSON.stringify(available_providers));
    }

    async function check_ongoing(vecdb_status) {
        let select_model_div = document.getElementById('vecdb-select-model');
        let indexing_progress_div = document.getElementById('vecdb-indexing-progress-row')
        let indexing_progress_bar = document.getElementById('vecdb-upload-files-progress-bar')
        let indexing_progress_span = document.getElementById('vecdb-upload-files-progress-span')
        let indexing_status_span = document.querySelector('.vecdb-indexing-status span')

        if (!vecdb_status) {
            return;
        }

        indexing_status_span.innerText = vecdb_status['status'];
        console.log(vecdb_status);

        if (vecdb_status['change_provider_flag']) {
            select_model_div.setAttribute('disabled', 'true');
        }

        const ongoing_indexing = vecdb_status["ongoing"]['indexing'];
        if (ongoing_indexing) {
            let progress_val = Math.round(parseInt(ongoing_indexing['file_n']) / parseInt(ongoing_indexing['total']) * 100);

            if (progress_val !== 100) {
                indexing_progress_div.removeAttribute("hidden");
                indexing_progress_bar.style.width = `${progress_val}%`;
                indexing_progress_span.innerText = `${ongoing_indexing['file_n']}/${ongoing_indexing['total']}`;
                select_model_div.setAttribute('disabled', "");
            } else {
                indexing_progress_div.hidden = true;
                select_model_div.removeAttribute('disabled');
            }
        } else {
            select_model_div.removeAttribute('disabled');
        }
    }

    await fetch('/tab-vecdb-status').then(
        async function (response) {
            if (response.ok) {
                return response.json();
            } else {
                return {}
            }
        }
    ).then(async (vecdb_status) => {
        await check_ongoing(vecdb_status);
        await vecdb_render_select_model(
            vecdb_status['available_providers'], vecdb_status['provider']
        )
    });
}


export async function tab_update_each_couple_of_seconds() {
    await render_vecdb_status()
}