export function finetune_info_factory(index, models_info, finetune_info, finetune_runs, multiple_loras) {
    let enabled_finetunes = [];

    if (models_info[index].hasOwnProperty('finetune_info') && models_info[index].finetune_info) {
        for (let run of models_info[index].finetune_info) {
            let enabled_finetune = document.createElement("div");
            enabled_finetune.dataset.run = run.run_id;
            enabled_finetune.dataset.checkpoint = run.checkpoint;
            enabled_finetune_factory(enabled_finetune, index);
            enabled_finetunes.push(enabled_finetune);
        }
    }

    let tech_msg = document.createElement("div");
    tech_msg.classList = "model-finetune-item-checkpoint";
    tech_msg.style = "font-size: 1em; margin: 0";

    if (!models_info[index].has_finetune) {
        tech_msg.innerText = "not supported";
        finetune_info.appendChild(tech_msg);
    } else if (finetune_runs.length == 0) {
        tech_msg.innerText = "no runs";
        finetune_info.appendChild(tech_msg);
    } else {
        let finetune_info_children = document.createElement("div");
        for (let child of enabled_finetunes) {
            finetune_info_children.appendChild(child);
        }
        finetune_info.appendChild(finetune_info_children);

        const selected_runs = models_info[index].finetune_info.map(run => run.run_id);
        const not_selected_runs = finetune_runs.filter(run => !selected_runs.includes(run.run_id));
        if (not_selected_runs.length > 0 && (selected_runs.length === 0 || multiple_loras || true)) {
            let add_finetune_btn = document.createElement("button");
            add_finetune_btn.classList = "btn btn-sm btn-outline-primary mt-1 add-finetune-btn";
            add_finetune_btn.style = "padding: 0 5px";
            add_finetune_btn.dataset.model = index;
            add_finetune_btn.innerText = 'Add Run';
            finetune_info.appendChild(add_finetune_btn);
        }
    }
}

function enabled_finetune_factory(enabled_finetune, model) {
    enabled_finetune.innerHTML = `
        <div class="model-finetune-item" style="display: flex; align-items: center; margin-bottom: 5px" data-run="${enabled_finetune.dataset.run}">
            <button class="btn btn-outline-danger btn-sm btn-remove-run" style="padding: 0 3px" 
            data-run="${enabled_finetune.dataset.run}" 
            data-checkpoint="${enabled_finetune.dataset.checkpoint}"
            data-model="${model}"
            >
                <i class="bi bi-trash3-fill" style="font-size: 1em"></i>
            </button>
            <div style="display: flex; flex-direction: column; margin-left: 10px;">
                <div class="model-finetune-item-run">
                    Run: ${enabled_finetune.dataset.run}
                </div>
                <div class="model-finetune-item-checkpoint">
                    Checkpoint: ${enabled_finetune.dataset.checkpoint}
                </div>
            </div>
        </div>
    `;
}

export function add_finetune_selectors_factory(finetune_configs_and_runs, models_info, model_name) {
    let existing_runs = []
    if (models_info[model_name].hasOwnProperty('finetune_info') && models_info[model_name].finetune_info) {
        existing_runs = models_info[model_name].finetune_info.map(run => run.run_id);
    }

    let el = document.createElement("div");
    let title = document.createElement("div");
    title.classList.add("model-finetune-item-run")
    title.style.marginTop = "10px"
    title.innerText = "Adding Run";
    el.appendChild(title);

    let dropdown_run = document.createElement("div");
    dropdown_run.classList.add("dropdown");

    let dropdown_btn_div = document.createElement("div");
    dropdown_btn_div.style = "display: flex; align-items: center;";
    let text = document.createElement("div");
    text.style = "font-size: 1em; margin-right: 5px;";
    text.innerText = "Run: ";
    dropdown_btn_div.appendChild(text);
    let dropdown_btn = document.createElement("button");
    dropdown_btn.id = "add-finetune-select-run-btn";
    dropdown_btn.classList = "btn dropdown-toggle";
    dropdown_btn.type = "button";
    dropdown_btn.dataset.toggle = "dropdown";
    dropdown_btn.style = "padding: 0; font-size: 1em; text-align: center;";
    dropdown_btn.innerHTML = "Select Run";
    dropdown_btn_div.appendChild(dropdown_btn);
    dropdown_run.appendChild(dropdown_btn_div);

    let dropdown_menu = document.createElement("div");
    dropdown_menu.id = "add-finetune-select-run-menu";
    dropdown_menu.classList.add("dropdown-menu");

    let runs = finetune_configs_and_runs.finetune_runs.filter(run => run.model_name === model_name && run.checkpoints.length !== 0 && !existing_runs.includes(run.run_id));
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
    control_btns_div.style = "display: flex; align-items: center; margin-top: 7px;";

    let add_btn = document.createElement("button");
    add_btn.id = "finetune-select-run-btn-add";
    add_btn.classList = "btn btn-outline-primary";
    add_btn.style = "padding: 0px 5px; margin-right: 7px;";
    add_btn.innerText = "add";

    let discard_btn = document.createElement("button");
    discard_btn.id = "finetune-select-run-btn-discard";
    discard_btn.classList = "btn btn-outline-secondary";
    discard_btn.style = "padding: 0px 5px;"
    discard_btn.innerText = "discard";

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
