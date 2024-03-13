import {general_error} from "../error.js";

export function finetune_switch_activate(finetune_model, mode, run_id, checkpoint) {
    let send_this = {
        "model": finetune_model,
        "mode": mode,
        "run_id": run_id,
        "checkpoint": checkpoint,
    }
    return fetch("/tab-host-modify-loras", {
        method: "POST",
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify(send_this)
    })
    .then(function (response) {
        if (!response.ok) {
            return response.json().then(function(json) {
                throw new Error(json.detail);
            });
        }
        return response.ok;
    })
    .catch(function (error) {
        console.log('tab-finetune-activate',error);
        general_error(error);
        return false;
    });
}

export function set_finetune_info_into_state(model_name, is_enabled) {
    const finetune_info = document.querySelector(`.model-finetune-info[data-model="${model_name}"]`);
    if (is_enabled) {
        finetune_info.style.pointerEvents = 'auto';
        finetune_info.style.opacity = '1';
    } else {
        finetune_info.style.pointerEvents = 'none';
        finetune_info.style.opacity = '0.5';
    }
}

export function add_finetune_selectors_factory(finetune_configs_and_runs, models_info, model_name) {
    let existing_runs = []
    if (models_info[model_name].hasOwnProperty('finetune_info') && models_info[model_name].finetune_info) {
        existing_runs = models_info[model_name].finetune_info.map(run => run.run_id);
    }

    let el = document.createElement("div");
    let title = document.createElement("div");
    title.classList.add("model-finetune-item-run")
    title.style.marginTop = "5px"
    title.style.marginBottom = "10px"
    title.innerText = "Finetune for " + model_name;
    el.appendChild(title);

    let dropdown_run = document.createElement("div");
    dropdown_run.classList.add("dropdown");

    let dropdown_btn_div = document.createElement("div");
    dropdown_btn_div.style = "display: flex; align-items: center;";
    let text = document.createElement("div");
    text.style = "font-size: 1em; margin-right: 5px;";
    text.innerText = "Finetune: ";
    dropdown_btn_div.appendChild(text);
    let dropdown_btn = document.createElement("button");
    dropdown_btn.id = "add-finetune-select-run-btn";
    dropdown_btn.classList = "btn dropdown-toggle";
    dropdown_btn.type = "button";
    dropdown_btn.dataset.toggle = "dropdown";
    dropdown_btn.style = "padding: 0; font-size: 1em; text-align: center;";
    dropdown_btn.innerHTML = "Select Finetune";
    dropdown_btn_div.appendChild(dropdown_btn);
    dropdown_run.appendChild(dropdown_btn_div);

    let dropdown_menu = document.createElement("div");
    dropdown_menu.id = "add-finetune-select-run-menu";
    dropdown_menu.classList.add("dropdown-menu");

    let runs = finetune_configs_and_runs.finetune_runs.filter(
        run => run.model_name === models_info[model_name].finetune_model
        && run.checkpoints.length !== 0 && !existing_runs.includes(run.run_id));
    for (let run of runs) {
        let child = document.createElement("a");
        child.setAttribute("class", "dropdown-item add-finetune-select-run-di");
        child.setAttribute("data-run", run.run_id);
        child.innerText = `${run.run_id}`;
        dropdown_menu.appendChild(child);
    }
    dropdown_run.appendChild(dropdown_menu)
    el.appendChild(dropdown_run.cloneNode(true));

    dropdown_run.innerHTML = "";
    dropdown_btn_div.innerHTML = "";
    dropdown_btn.innerHTML = "";
    dropdown_menu.innerHTML = "";
    text.innerText = "Checkpoint: ";
    dropdown_btn_div.appendChild(text);
    dropdown_btn.innerText = "Best (Auto)";
    dropdown_btn.id = "add-finetune-select-checkpoint-btn";
    dropdown_btn.disabled = true;
    dropdown_btn.style.border = "none";
    dropdown_btn_div.appendChild(dropdown_btn);
    dropdown_run.appendChild(dropdown_btn_div);
    dropdown_menu.id = "add-finetune-select-checkpoint-menu";
    dropdown_run.appendChild(dropdown_menu);

    el.appendChild(dropdown_run.cloneNode(true));

    let control_btns_div = document.createElement("div");
    control_btns_div.style = "display: flex; align-items: center; margin-top: 14px;";

    let add_btn = document.createElement("button");
    add_btn.id = "finetune-select-run-btn-add";
    add_btn.classList = "btn btn-outline-primary";
    add_btn.style = "padding: 0px 5px; margin-right: 7px;";
    add_btn.innerText = "Ok";

    let discard_btn = document.createElement("button");
    discard_btn.id = "finetune-select-run-btn-discard";
    discard_btn.classList = "btn btn-outline-secondary";
    discard_btn.style = "padding: 0px 5px;"
    discard_btn.innerText = "Cancel";

    control_btns_div.appendChild(add_btn);
    control_btns_div.appendChild(discard_btn);

    el.appendChild(control_btns_div);

    return el;
}

export function update_checkpoints_list(finetune_configs_and_runs, finetune_select_checkpoint_btn, run_id, checkpoint_menu) {
    let runs = finetune_configs_and_runs.finetune_runs.filter(run => run.run_id === run_id);
    if (runs.length === 0) {
        return;
    }
    let run = runs[0];
    let checkpoints = run.checkpoints;
    let best_checkpoint_id = run.best_checkpoint.best_checkpoint_id ? run.best_checkpoint.best_checkpoint_id : null;
    finetune_select_checkpoint_btn.dataset.name = run.best_checkpoint.best_checkpoint_id;
    if (run.best_checkpoint.best_checkpoint_id) {
        finetune_select_checkpoint_btn.innerText = `${run.best_checkpoint.best_checkpoint_id} (best)`;
    } else {
        finetune_select_checkpoint_btn.innerText = 'Best (Auto)';
    }

    checkpoint_menu.innerHTML = "";
    checkpoint_menu.dataset.best_checkpoint_id = best_checkpoint_id;
    for (let c of checkpoints) {
        let child = document.createElement("a");
        child.setAttribute("class", "dropdown-item add-finetune-select-checkpoint-di");
        child.setAttribute("data-name", c.checkpoint_name);
        child.innerText = `${c.checkpoint_name}` + (c.checkpoint_name === best_checkpoint_id ? ' (best)' : '');
        checkpoint_menu.appendChild(child);
    }
}
