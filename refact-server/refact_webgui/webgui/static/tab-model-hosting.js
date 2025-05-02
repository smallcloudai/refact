import {general_error} from './error.js';
import {get_finetune_config_and_runs} from './tab-finetune.js';
import {add_finetune_selectors_factory, update_checkpoints_list, set_finetune_info_into_state, finetune_switch_activate} from './utils/tab-model-hosting-utils.js';
import {get_spinner} from "./utils/utils.js";


let device_popup = false;
let models_data = null;
let finetune_configs_and_runs;
let force_render_models_assigned = false;
const highlight_python = code => {
    // Process each type of syntax in sequence to avoid overlaps
    return code
        // Comments first (whole line)
        .replace(/(#.*)$/gm, '<span style="color: #6e7781;">$1</span>')
        // Strings (both single and double quoted)
        .replace(/(['"])(.*?)\1/g, '<span style="color: #0a3069;">$1$2$1</span>')
        // Keywords
        .replace(/\b(def|class|return|import|from|if|else|elif|for|while|try|except|with|as|pass|lambda|yield|async|await|raise|break|continue|in|is|not|and|or|None|True|False)\b/g, 
            '<span style="color: #cf222e;">$1</span>')
        // Numbers
        .replace(/\b(\d+)\b/g, '<span style="color: #0550ae;">$1</span>')
        // Decorators
        .replace(/(@[\w.]+)/g, '<span style="color: #953800;">$1</span>');
};

function upload_weights_code_snippet(model_path) {
    const code_snippet = `def download_model_tar(repo_id: str) -> str:
    import tarfile, tempfile
    from os import path, getcwd, listdir
    from huggingface_hub import snapshot_download

    tar_filename = path.join(getcwd(), f"{repo_id.replace('/', '--')}.tar")
    with tempfile.TemporaryDirectory() as tmpdir:
        snapshot_download(repo_id=repo_id, cache_dir=tmpdir)
        model_dirs = [f for f in listdir(tmpdir) if f.startswith("models--")]
        assert model_dirs, f"No models downloaded for {repo_id}"
        with tarfile.open(tar_filename, "w") as tar:
            for model_dir in model_dirs:
                tar.add(path.join(tmpdir, model_dir), model_dir)
    return tar_filename

model_path = "${model_path}"
tar_filename = download_model_tar(model_path)
print(f"Model {model_path} loaded and packed into {tar_filename}")
`;
    const highlighted_code_snippet = highlight_python(code_snippet);
    return `<pre><code id="weights-code">${highlighted_code_snippet}</code></pre>`;
}

function update_finetune_configs_and_runs() {
    get_finetune_config_and_runs().then((data) => {
        if (!data) {
            return;
        }
        finetune_configs_and_runs = data;
    })
}

function get_devices() {
    fetch("/tab-host-have-devices")
    .then(function(response) {
        return response.json();
    })
    .then(function(data) {
        render_devices(data);
    })
   .catch(function(error) {
        console.log('tab-host-have-devices',error);
        general_error(error);
    });
}

function render_device(device_image_class, device_id, name, mem_used_mb, mem_total_mb, temp_celsius, statuses) {
    const device_div = document.createElement('div');
    device_div.classList.add('device-item');
    device_div.setAttribute('device', device_id);

    const device_image = document.createElement("div");
    device_image.classList.add(device_image_class);

    const device_content = document.createElement('div');
    device_content.classList.add('device-content');

    const device_name = document.createElement("h3");
    device_name.classList.add('device-title');
    device_name.innerHTML = name;

    const device_mem = document.createElement("div");
    device_mem.classList.add('device-mem');
    const used_gb = format_memory(mem_used_mb);
    const total_gb = format_memory(mem_total_mb);
    const used_mem = Math.round(mem_used_mb / (mem_total_mb / 100));
    device_mem.innerHTML = `
        <b>Mem</b>
        <div class="device-mem-wrap">
            <div class="device-mem-bar">
                <span style="width: ${used_mem}%"></span>
            </div>
            ${used_gb}/${total_gb} GB
        </div>
    `;

    const device_temp = document.createElement("div");
    device_temp.classList.add('device-temp');
    if (temp_celsius < 0) {
        device_temp.innerHTML = `<b>Temp</b> N/A`;
    } else {
        device_temp.innerHTML = `<b>Temp</b>` + temp_celsius + '°C';
    }

    device_content.appendChild(device_name);
    device_content.appendChild(device_mem);
    device_content.appendChild(device_temp);
    statuses.forEach(status => {
        const device_status = document.createElement("div");
        device_status.classList.add('device-status');
        device_status.innerHTML += `<div><b>Command</b>${status.command}</div>`;
        device_status.innerHTML += `<div><b>Status</b>${status.status}</div>`;

        const device_command = document.createElement("div");
        device_command.classList.add('device-command');
        device_command.innerHTML = `<span class="device-current-status">${status.status}</span>`;
        device_command.appendChild(device_status);
        device_command.addEventListener('mouseover',function(e) {
            device_popup = true;
            this.querySelector('.device-status').classList.add('device-status-visible');
        });
        device_command.addEventListener('mouseout',function(e) {
            device_popup = false;
            this.querySelector('.device-status').classList.remove('device-status-visible');
        });
        if(!status.status || status.status === '') {
            device_command.classList.add('device-status-invisible');
        }
        device_content.appendChild(device_command);
    });

    device_div.appendChild(device_image);
    device_div.appendChild(device_content);

    return device_div;
}

function render_devices(data) {
    if(device_popup) {
        return;
    }

    const cpu_div = render_device(
        'devices-cpu',
        data.cpu.id,
        data.cpu.name,
        data.cpu.mem_used_mb,
        data.cpu.mem_total_mb,
        data.cpu.temp_celsius,
        data.cpu.statuses,
    );

    const cpu_pane = document.querySelector('.cpu-pane');
    cpu_pane.innerHTML = '';
    cpu_pane.appendChild(cpu_div);

    if(data.gpus.length == 0) {
        document.querySelector('.gpus-pane').style.display = 'none';
    } else {
        document.querySelector('.gpus-pane').style.display = 'div';
    }
    const gpus_list = document.querySelector('.gpus-list');
    gpus_list.innerHTML = '';
    data.gpus.forEach(element => {
        const row = render_device(
            'devices-card',
            element.id,
            element.name,
            element.mem_used_mb,
            element.mem_total_mb,
            element.temp_celsius,
            element.statuses,
        );
        gpus_list.appendChild(row);
    });
}

// Function removed as it's no longer needed

function get_models()
{
    fetch("/tab-host-models-get")
    .then(function(response) {
        return response.json();
    })
    .then(function(data) {
        models_data = data;
        render_models_assigned(data.model_assign);

        const more_gpus_notification = document.querySelector('.model-hosting-error');
        if(data.hasOwnProperty('more_models_than_gpus') && data.more_models_than_gpus) {
            more_gpus_notification.classList.remove('d-none');
        } else {
            more_gpus_notification.classList.add('d-none');
        }
        const required_memory_exceed_available = document.querySelector('.model-memory-error');
        if(data.hasOwnProperty('required_memory_exceed_available') && data.required_memory_exceed_available) {
            required_memory_exceed_available.classList.remove('d-none');
        } else {
            required_memory_exceed_available.classList.add('d-none');
        }
    })
    .catch(function(error) {
        console.log('tab-host-models-get', error);
        general_error(error);
    });
}

function save_model_assigned() {
    const data = {
        model_assign: {
            ...models_data.model_assign,
        },
    };

    fetch("/tab-host-models-assign", {
        method: "POST",
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify(data)
    })
    .then(function (response) {
        if (!response.ok) {
            return response.json().then(error => { throw error });
        }
        return response.json();
    })
    .then(function (data) {
        get_models();
    })
    .catch(function (error) {
        console.log('tab-host-models-assign', error.detail);
        general_error(error.detail);
        get_models();
    });
}

function render_models_assigned(models) {
    const models_info = models_data.models.reduce(function(obj, item) {
      obj[item["name"]] = item;
      return obj;
    }, {});
    const models_table = document.querySelector('.table-assigned-models tbody');

    if (models_table.dataset.hash == CryptoJS.MD5(JSON.stringify(models)) && !force_render_models_assigned) {
        return;
    }
    force_render_models_assigned = false;
    models_table.dataset.hash = CryptoJS.MD5(JSON.stringify(models));

    models_table.innerHTML = '';
    let models_index = 0;
    for(let index in models) {
        const row = document.createElement('tr');
        row.setAttribute('data-model',index);
        let model_name = document.createElement("td");
        model_name.style.width = "20%";
        let context = document.createElement("td");
        context.style.width = "15%";
        let finetune_info = document.createElement("td");
        finetune_info.style.width = "35%";
        let select_gpus = document.createElement("td");
        select_gpus.style.width = "15%";
        let gpus_share = document.createElement("td");
        gpus_share.style.width = "10%";
        let del = document.createElement("td");
        del.style.width = "5%";

        model_name.textContent = index;
        finetune_info.classList.add('model-finetune-info');
        finetune_info.dataset.model = index;

        if(models_info[index].is_deprecated) {
            const deprecated_notice = document.createElement('span');
            deprecated_notice.classList.add('deprecated-badge','badge','rounded-pill','text-dark');
            deprecated_notice.setAttribute('data-bs-toggle','tooltip');
            deprecated_notice.setAttribute('data-bs-placement','top');
            deprecated_notice.setAttribute('title','Deprecated: this model will be removed in future releases.');
            deprecated_notice.textContent = 'Deprecated';
            model_name.innerHTML = index;
            model_name.appendChild(deprecated_notice);
            new bootstrap.Tooltip(deprecated_notice);
        }

        let btn_group = document.createElement("div");
        btn_group.classList.add('btn-group');
        btn_group.role = 'group';
        const model_n_ctx = models_data.model_assign[index].n_ctx
        if (models_info[index].available_n_ctx && models_info[index].available_n_ctx.length > 0) {
            const context_size = models_info[index].available_n_ctx;
            const context_input = document.createElement("select");
            context_input.classList.add('form-select','form-select-sm');
            context_size.forEach(element => {
                const context_option = document.createElement("option");
                context_option.setAttribute('value',element);
                context_option.textContent = element;
                if(element === model_n_ctx) {
                    context_option.setAttribute('selected','selected');
                }
                context_input.appendChild(context_option);
            });
            context_input.addEventListener('change', function() {
                models_data.model_assign[index].n_ctx = Number(this.value);
                save_model_assigned();
            });
            context.appendChild(context_input);
        }
        if (models_info[index].available_n_ctx && models_info[index].available_n_ctx.length == 0) {
            context.innerHTML = `<span class="default-context">${models_info[index].default_n_ctx}</span>`;
        }

        let finetune_runs = [];
        if (finetune_configs_and_runs) {
            finetune_runs = finetune_configs_and_runs.finetune_runs.filter(
                run => run.model_name === models_info[index].finetune_model
                && run.checkpoints.length !== 0);
            if (models_info[index].finetune_model !== index) {
                finetune_runs = finetune_runs.filter(run => !run.deprecated);
            }
        } else {
            force_render_models_assigned = true;
        }
        finetune_info_factory(index, models_info, finetune_info, finetune_runs, models_data.multiple_loras);

         if (models_info[index].available_shards.length > 1) {
            const select_gpus_div = document.createElement("div");
            select_gpus_div.setAttribute("class", "btn-group btn-group-sm");
            select_gpus_div.setAttribute("role", "group");
            select_gpus_div.setAttribute("aria-label", "basic radio toggle button group");

            models_info[index].available_shards.forEach((gpus_shard_n) => {
                const input_name = `gpu-${index}`;
                const input_id = `${input_name}-${gpus_shard_n}`;

                const input = document.createElement("input");
                input.setAttribute("type", "radio");
                input.setAttribute("class", "gpu-switch btn-check");
                input.setAttribute("name", input_name);
                input.setAttribute("id", input_id);
                if (models_data.model_assign[index].gpus_shard === gpus_shard_n) {
                    input.checked = true;
                }

                const label = document.createElement("label");
                label.setAttribute("class", "btn btn-outline-primary");
                label.setAttribute("for", input_id);
                label.innerHTML = gpus_shard_n;

                input.addEventListener('change', () => {
                    models_data.model_assign[index].gpus_shard = gpus_shard_n;
                    save_model_assigned();
                });

                select_gpus_div.appendChild(input);
                select_gpus_div.appendChild(label);
            });
            select_gpus.appendChild(select_gpus_div);
        }

        if(models_info[index].has_share_gpu) {
            const gpus_checkbox = document.createElement("input");
            gpus_checkbox.setAttribute('type','checkbox');
            gpus_checkbox.setAttribute('value',index);
            gpus_checkbox.setAttribute('name',`share-${index}`);
            gpus_checkbox.classList.add('form-check-input');
            if(models_data.model_assign[index].share_gpu) {
                gpus_checkbox.checked = true;
            } 
            gpus_checkbox.addEventListener('change', function() {
                if(this.checked) {
                    models_data.model_assign[index].share_gpu = true;
                } else {
                    models_data.model_assign[index].share_gpu = false;
                }
                save_model_assigned();
            });
            gpus_share.appendChild(gpus_checkbox);
        }

        const del_button = document.createElement("button");
        del_button.innerHTML = `<i class="bi bi-trash3-fill"></i>`;
        del_button.dataset.model = index;
        del_button.addEventListener('click', function() {
            delete models_data.model_assign[index];
            save_model_assigned();
        });
        del_button.classList.add('model-remove','btn','btn-outline-danger');
        del.appendChild(del_button);

        row.appendChild(model_name);
        row.appendChild(context);
        row.appendChild(finetune_info);
        row.appendChild(select_gpus);
        row.appendChild(gpus_share);
        row.appendChild(del);
        models_table.appendChild(row);
        models_index++;
    }
}

let toggle_menu_display = (menu) => {
    menu.style.display = (menu.style.display === 'none' || menu.style.display === '') ? 'block' : 'none';
}

function on_add_finetune_btn_click(el, event, models_info) {
    get_models();
    const target = event.currentTarget;

    let finetune_info = document.querySelector(`.model-finetune-info[data-model="${target.dataset.model}"]`);

    let dropdownPanel = document.createElement('div');
    dropdownPanel.classList.add('dropdown-panel-add-finetune')
    let finetune_selectors = add_finetune_selectors_factory(finetune_configs_and_runs, models_info, target.dataset.model);

    dropdownPanel.appendChild(finetune_selectors);

    for (let old_panel of document.querySelectorAll('.dropdown-panel-add-finetune')) {
        old_panel.remove();
    }
    finetune_info.appendChild(dropdownPanel);
    dropdownPanel.style.display = 'block';

    let finetune_add_btn = document.querySelector("#finetune-select-run-btn-add");
    finetune_add_btn.disabled = true;

    let finetune_select_run_btn = document.getElementById('add-finetune-select-run-btn');

    let run_menu = document.getElementById('add-finetune-select-run-menu');

    let finetune_select_checkpoint_btn = document.getElementById('add-finetune-select-checkpoint-btn');
    let checkpoint_menu = document.getElementById('add-finetune-select-checkpoint-menu');


    finetune_select_run_btn.addEventListener('click', (event) => {
        toggle_menu_display(run_menu);
    });

    finetune_select_checkpoint_btn.addEventListener('click', (event) => {
        toggle_menu_display(checkpoint_menu);
    });

    document.addEventListener('click', function(event) {
        if (!run_menu.contains(event.target) && event.target !== finetune_select_run_btn) {
            run_menu.style.display = 'none';
        }
        if (!checkpoint_menu.contains(event.target) && event.target !== finetune_select_checkpoint_btn) {
            checkpoint_menu.style.display = 'none';
        }
        // if (!dropdownPanel.contains(event.target) && event.currentTarget !== target) {
        //     dropdownPanel.style.display = 'none';
        // }
    });

    document.querySelectorAll('.add-finetune-select-run-di').forEach(element => {
       element.addEventListener('click', function(e) {
           finetune_select_run_btn.innerText = `${e.currentTarget.innerText}`;
           finetune_select_run_btn.dataset.run = e.currentTarget.dataset.run;
           run_menu.style.display = 'none';

           update_checkpoints_list(finetune_configs_and_runs, finetune_select_checkpoint_btn, e.currentTarget.dataset.run, checkpoint_menu);

           finetune_select_checkpoint_btn.disabled = false;
           finetune_add_btn.disabled = false;

           document.querySelectorAll('.add-finetune-select-checkpoint-di').forEach(element => {
                element.addEventListener('click', (e) => {
                    finetune_select_checkpoint_btn.innerText = `${e.currentTarget.innerText}`;
                    finetune_select_checkpoint_btn.dataset.name = e.currentTarget.dataset.name;
                    checkpoint_menu.style.display = 'none';
                });
            });
       });
    });
    let discard_btn = document.querySelector("#finetune-select-run-btn-discard");

    finetune_add_btn.addEventListener('click', (el) => {
        discard_btn.disabled = true;
        const spinner = get_spinner();
        el.target.replaceWith(spinner);

        finetune_switch_activate(
            target.dataset.model,
            "add",
            finetune_select_run_btn.dataset.run,
            finetune_select_checkpoint_btn.dataset.name
        ).then((is_ok) => {
        if (!is_ok) {
            discard_btn.disabled = false;
            spinner.replaceWith(el.target);
        } else {
            force_render_models_assigned = true;
        }
    });

    });
    document.querySelector("#finetune-select-run-btn-discard").addEventListener('click', (el) => {
        finetune_selectors.remove();
        dropdownPanel.style.display = 'none';
        target.disabled = false;
    });

}

function init_tooltips() {
    const tooltipTriggerList = [].slice.call(document.querySelectorAll('[data-bs-toggle="tooltip"]'));
    tooltipTriggerList.forEach(tooltipTriggerEl => {
        if (!tooltipTriggerEl.hasAttribute('data-tooltip-initialized')) {
            new bootstrap.Tooltip(tooltipTriggerEl);
            tooltipTriggerEl.setAttribute('data-tooltip-initialized', 'true');
        }
    });
}

function render_models(models) {
    const models_table = document.querySelector('.table-models tbody');
    models_table.innerHTML = '';
    models.models.sort((a, b) => a.name.localeCompare(b.name, undefined, { sensitivity: 'base' }));
    let models_tree = models.models.reduce((result, item) => {
        const name_parts = item.name.split('/');
        const group_name = name_parts[0];
        if (!result[group_name]) {
          result[group_name] = [];
        }
        result[group_name].push(item);
        return result;
    }, {});
    for (const group_name in models_tree) {
        if (models_tree.hasOwnProperty(group_name)) {
            models_tree[group_name].sort((a, b) => {
            return a.name.localeCompare(b.name);
          });
        }
    }
    for (const [key, value] of Object.entries(models_tree)) {
        const row = document.createElement('tr');
        row.classList.add('model-row');
        row.setAttribute('data-model',key);
        const model_name = document.createElement("td");
        const model_span = document.createElement('span');
        const has_completion = document.createElement("td");
        const has_finetune = document.createElement("td");
        const has_chat = document.createElement("td");
        const model_weights = document.createElement("td");
        model_span.textContent = key;
        model_span.classList.add('model-span');
        model_name.appendChild(model_span);
        row.appendChild(model_name);
        row.appendChild(has_completion);
        row.appendChild(has_finetune);
        row.appendChild(has_chat);
        row.appendChild(model_weights);
        models_table.appendChild(row);
        value.forEach(element => {
            const row = document.createElement('tr');
            row.dataset.model = element.name;
            row.dataset.default_gpus_shard = element.available_shards.length > 0 ? element.available_shards[0] : 1;
            row.setAttribute('data-parent',key);
            row.classList.add('modelsub-row');
            const model_name = document.createElement("td");
            const has_completion = document.createElement("td");
            const has_finetune = document.createElement("td");
            const has_chat = document.createElement("td");
            const model_weights = document.createElement("td");
            model_name.innerHTML = element.name;
            if(element.repo_status == 'gated') {
                model_name.innerHTML = '';
                const model_name_div = document.createElement('div');
                model_name_div.classList.add('modelsub-name');
                const model_holder_div = document.createElement('div');
                model_holder_div.innerHTML = element.name;
                const model_info_div = document.createElement('div');
                model_info_div.classList.add('modelsub-info');
                model_info_div.innerHTML = `<b>Gated models downloading requires:</b><br />
                1. Huggingface Hub token in <span class="modelinfo-settings">settings.</span><br />
                2. Accept conditions at <a target="_blank" href="${element.repo_url}">model's page.</a><br />
                Make sure that you have access to this model.<br />
                More info about gated model <a target="_blank" href="https://huggingface.co/docs/hub/en/models-gated">here</a>.`;
                model_name_div.appendChild(model_holder_div);
                model_name_div.appendChild(model_info_div);
                model_name.appendChild(model_name_div);
            }
            if(element.is_deprecated) {
                const deprecated_notice = document.createElement('span');
                deprecated_notice.classList.add('deprecated-badge','badge','rounded-pill','text-dark');
                deprecated_notice.setAttribute('data-bs-toggle','tooltip');
                deprecated_notice.setAttribute('data-bs-placement','top');
                deprecated_notice.setAttribute('title','Deprecated: this model will be removed in future releases.');
                deprecated_notice.textContent = 'Deprecated';
                model_name.appendChild(deprecated_notice);
                new bootstrap.Tooltip(deprecated_notice);
            }
            if(element.repo_url) {
                const repo_badge = document.createElement('a');
                repo_badge.classList.add('repo-badge','badge','rounded-pill','text-dark');
                repo_badge.setAttribute('href',element.repo_url);
                repo_badge.setAttribute('target','_blank');
                repo_badge.textContent = new URL(element.repo_url).hostname;
                model_name.appendChild(repo_badge);
            }
            if(element.hasOwnProperty('has_completion')) {
                has_completion.innerHTML = element.has_completion ? '<i class="bi bi-check"></i>' : '';
            }
            if(element.hasOwnProperty('has_finetune')) {
                has_finetune.innerHTML = element.has_finetune ? '<i class="bi bi-check"></i>' : '';
            }
            if(element.hasOwnProperty('has_chat')) {
                has_chat.innerHTML = element.has_chat ? '<i class="bi bi-check"></i>' : '';
            }
            const has_weights_loaded = element.hasOwnProperty("has_weights_loaded") && element.has_weights_loaded;
            if (element.hasOwnProperty("is_hf_offline") && element.is_hf_offline) {
                const model_weights_are_loaded = document.createElement('div');
                model_weights_are_loaded.innerHTML = '<i data-bs-toggle="tooltip" data-bs-placement="top" title="Weights are loaded" class="bi bi-save"></i>';
                model_weights_are_loaded.style.visibility = has_weights_loaded ? "visible" : "hidden";

                const model_weights_upload_button = document.createElement('button');
                model_weights_upload_button.classList.add('badge','bg-primary','model-weights-button');
                model_weights_upload_button.value = 'Upload weights';
                model_weights_upload_button.title = 'Upload weights manually';
                model_weights_upload_button.dataset.bsToggle = 'tooltip';
                model_weights_upload_button.dataset.bsPlacement = 'top';
                model_weights_upload_button.innerHTML = '<i class="bi bi-cloud-plus"></i> Upload weights';
                model_weights_upload_button.dataset.model_path = element.model_path;

                const model_weights_info_div = document.createElement('div');
                model_weights_info_div.classList.add('model-weights-info');;
                model_weights_info_div.appendChild(model_weights_are_loaded);
                model_weights_info_div.appendChild(model_weights_upload_button);
                model_weights.appendChild(model_weights_info_div);
            } else {
                const model_weights_info_div = document.createElement('div');
                if (has_weights_loaded) {
                    model_weights_info_div.innerHTML = '<i data-bs-toggle="tooltip" data-bs-placement="top" title="Weights are loaded" class="bi bi-save"></i>';
                }
                model_weights.appendChild(model_weights_info_div);
            }
            row.appendChild(model_name);
            row.appendChild(has_completion);
            row.appendChild(has_finetune);
            row.appendChild(has_chat);
            row.appendChild(model_weights);
            models_table.appendChild(row);
            const add_model_modal = bootstrap.Modal.getOrCreateInstance(document.getElementById('add-model-modal'));
            row.addEventListener('click', function(e) {
                if(e.target.classList.contains('modelinfo-settings')) {
                    document.querySelector('button[data-tab="settings"]').click();
                    const add_model_modal = bootstrap.Modal.getOrCreateInstance(document.getElementById('add-model-modal'));
                    add_model_modal.hide();
                } else if (e.target.tagName.toLowerCase() === 'a') {
                    e.preventDefault();
                    const href = e.target.getAttribute('href');
                    window.open(href, '_blank');
                } else if (e.target.tagName.toLowerCase() === 'button') {
                    const upload_weights_modal = bootstrap.Modal.getOrCreateInstance(document.getElementById('upload-weights-modal'));
                    const code_snippet_wrapper = document.querySelector('.weights-modal-code');
                    code_snippet_wrapper.innerHTML = upload_weights_code_snippet(e.target.dataset.model_path);
                    document.querySelector('.weights-modal-info').innerHTML = `
                        <p>Download model weights using given code example.</p>
                        <p>Next upload obtained archive to the server.</p>
                    `;
                    document.querySelector('label[for="model_weights"] span').innerHTML = e.target.dataset.model_path;
                    const weightsCode = document.querySelector('#weights-code');
                    if (weightsCode) {
                        weightsCode.addEventListener('click', function() {
                            navigator.clipboard.writeText(this.textContent || this.innerText);
                        });
                    }
                    add_model_modal.hide();
                    upload_weights_modal.show();
                } else {
                    const model_name = this.dataset.model;
                    const default_gpus_shard = this.dataset.default_gpus_shard;
                    models_data.model_assign[model_name] = {
                        gpus_shard: default_gpus_shard,
                        n_ctx: element.default_n_ctx,
                    };
                    save_model_assigned();
                    add_model_modal.hide();
                }
            });
        });
        row.addEventListener('click', function(e) {
            this.classList.toggle('row-active');
            const model_name = this.dataset.model;
            const sub_rows = document.querySelectorAll('[data-parent="'+model_name+'"]');
            sub_rows.forEach(sub_row => {
                sub_row.style.display = sub_row.style.display === 'table-row'? 'none' : 'table-row';
            });
        });
    }
    init_tooltips();
}

function finetune_info_factory(index, models_info, finetune_info, finetune_runs, multiple_loras) {
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
    tech_msg.classList.add("tech-msg");

    if (!models_info[index].has_finetune) {
        tech_msg.innerText = "Finetune not supported";
        finetune_info.appendChild(tech_msg);
    } else if (finetune_runs.length == 0) {
        tech_msg.innerText = "no finetunes available";
        finetune_info.appendChild(tech_msg);
    } else {
        let finetune_info_children = document.createElement("div");
        for (let child of enabled_finetunes) {
            finetune_info_children.appendChild(child);
        }
        finetune_info.appendChild(finetune_info_children);

        const selected_runs = models_info[index].finetune_info.map(run => run.run_id);
        const not_selected_runs = finetune_runs.filter(run => !selected_runs.includes(run.run_id));
        if (not_selected_runs.length > 0 && (selected_runs.length === 0 || multiple_loras)) {
            let add_finetune_btn = document.createElement("button");
            add_finetune_btn.classList = "btn btn-sm btn-outline-primary mt-1 add-finetune-btn";
            add_finetune_btn.dataset.model = index;
            add_finetune_btn.innerText = 'Add Finetune';
            finetune_info.appendChild(add_finetune_btn);

            add_finetune_btn.addEventListener('click', (event) => {
                on_add_finetune_btn_click(add_finetune_btn, event, models_info);
            });
        }
    }
}

// TODO: doesn't work well after one upload
function upload_weights() {
    const file = document.querySelector('#model_weights').files[0];
    const weights_modal_submit = document.querySelector('.weights-modal-submit');
    
    if (!file) {
        general_error({ detail: "Please select a file first" });
        return;
    }

    const formData = new FormData();
    formData.append('file', file);
   
    const progressContainer = document.createElement('div');
    progressContainer.className = 'progress';
    progressContainer.style.height = '20px';
    progressContainer.style.marginTop = '20px';

    const progressBar = document.createElement('div');
    progressBar.className = 'progress-bar';
    progressBar.role = 'progressbar';
    progressBar.style.width = '0%';
    progressBar.setAttribute('aria-valuenow', '0');
    progressBar.setAttribute('aria-valuemin', '0');
    progressBar.setAttribute('aria-valuemax', '100');
    
    progressContainer.appendChild(progressBar);

    document.querySelector('.weights-progress').replaceWith(progressContainer);

    const xhr = new XMLHttpRequest();
    
    xhr.upload.addEventListener('progress', (event) => {
        if (event.lengthComputable) {
            const percentComplete = (event.loaded / event.total) * 100;
            progressBar.style.width = percentComplete + '%';
            progressBar.setAttribute('aria-valuenow', percentComplete);
            progressBar.textContent = Math.round(percentComplete) + '%';
        }
    });

    const cleanupForm = () => {
        document.querySelector('#model_weights').value = '';
        const emptyProgress = document.createElement('div');
        emptyProgress.className = 'weights-progress';
        progressContainer.replaceWith(emptyProgress);
        weights_modal_submit.disabled = false;
    };

    xhr.onload = function() {
        if (xhr.status === 200) {
            const data = JSON.parse(xhr.responseText);
            const upload_weights_modal = bootstrap.Modal.getOrCreateInstance(document.getElementById('upload-weights-modal'));
            cleanupForm();
            upload_weights_modal.hide();
            get_models();
        } else {
            try {
                const error = JSON.parse(xhr.responseText);
                general_error(error);
            } catch (e) {
                general_error({ detail: "Upload failed" });
            }
            console.log('model-weights-upload error:', xhr.status, xhr.responseText);
            cleanupForm();
        }
    };

    xhr.onerror = function() {
        console.log('model-weights-upload network error');
        general_error({ detail: "Network error occurred" });
        cleanupForm();
    };

    xhr.open('POST', '/model-weights-upload', true);
    xhr.send(formData);
}

function enabled_finetune_factory(enabled_finetune, model) {
    let outer_div = document.createElement('div');
    let inner_div = document.createElement('div');
    let upper_row_div = document.createElement('div');
    let run_div = document.createElement('div');
    let button_div = document.createElement('div');
    let button = document.createElement('button');
    let checkpoint_div = document.createElement('div');

    outer_div.classList.add('model-finetune-item');
    inner_div.classList.add('model-finetune-item-inner');
    upper_row_div.classList.add('model-finetune-item-upper-row');
    run_div.classList.add('model-finetune-item-run');
    button_div.style.marginLeft = 'auto';
    button.classList.add('btn-remove-run');
    checkpoint_div.classList.add('model-finetune-item-checkpoint');

    outer_div.dataset.run = enabled_finetune.dataset.run;
    run_div.textContent = enabled_finetune.dataset.run;
    button.dataset.run = enabled_finetune.dataset.run;
    button.dataset.checkpoint = enabled_finetune.dataset.checkpoint;
    button.dataset.model = model;
    button.textContent = 'x';
    checkpoint_div.textContent = 'Checkpoint: ' + enabled_finetune.dataset.checkpoint;

    upper_row_div.appendChild(run_div);
    button_div.appendChild(button);
    upper_row_div.appendChild(button_div);
    inner_div.appendChild(upper_row_div);
    inner_div.appendChild(checkpoint_div);
    outer_div.appendChild(inner_div);

    enabled_finetune.innerHTML = '';
    enabled_finetune.appendChild(outer_div);

    button.addEventListener('click', () => {
        const spinner = get_spinner();
        spinner.style.padding = '0';
        spinner.style.margin = '0';
        spinner.style.scale = '0.5';
        spinner.style.position = 'relative';

        set_finetune_info_into_state(button.dataset.model, false);
        button.replaceWith(spinner);

        finetune_switch_activate(
            button.dataset.model,
            "remove",
            button.dataset.run,
            button.dataset.checkpoint
        ).then((is_ok) => {
            if (!is_ok) {
                set_finetune_info_into_state(button.dataset.model, true);
                spinner.replaceWith(button);
            } else {
                force_render_models_assigned = true;
            }
        });
    });
}

function format_memory(memory_in_mb, decimalPlaces = 2) {
    return (memory_in_mb / 1024).toFixed(decimalPlaces);
}

export async function init(general_error) {
    let req = await fetch('/tab-model-hosting.html');
    document.querySelector('#model-hosting').innerHTML = await req.text();
    get_devices();
    update_finetune_configs_and_runs();
    get_models();
    const add_model_modal = document.getElementById('add-model-modal');
    add_model_modal.addEventListener('show.bs.modal', function () {
        render_models(models_data);
    });
//    const redirect2credentials = document.getElementById('redirect2credentials');
//    redirect2credentials.addEventListener('click', function() {
//        document.querySelector(`[data-tab=${redirect2credentials.getAttribute('data-tab')}]`).click();
//    });
    const weights_modal_submit = document.querySelector('.weights-modal-submit');
    weights_modal_submit.addEventListener('click', function() {
        const fileInput = document.querySelector('#model_weights');
        if (fileInput.files.length > 0) {
            weights_modal_submit.disabled = true;
            upload_weights(null, fileInput.files[0]);
        } else {
            general_error('Please select a file to upload');
        }
    });
    const code_snippet_wrapper = document.querySelector('.weights-modal-code');
    if (code_snippet_wrapper) {
        code_snippet_wrapper.addEventListener("click", function () {
            const code_snippet = document.querySelector('#weights-code');
            if (code_snippet) {
                const text = code_snippet.innerText || code_snippet.textContent;
                navigator.clipboard.writeText(text);
            }
        });
    }
    // const enable_chat_gpt_switch = document.getElementById('enable_chat_gpt');
}

export function tab_switched_here() {
    get_devices();
    update_finetune_configs_and_runs();
    get_models();
}

export function tab_switched_away() {
}

export function tab_update_each_couple_of_seconds() {
    get_devices();
    update_finetune_configs_and_runs();
    if (force_render_models_assigned) {
        get_models();
    }
}
