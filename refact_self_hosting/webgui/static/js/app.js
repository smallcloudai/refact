import * as model_hosting_tab from './tab-model-hosting.js';
import * as upload_tab from './tab-upload.js';
import * as finetune_tab from './tab-finetune.js';
import * as access_control_tab from './tab-access-contol.js';
import * as server_log_tab from './tab-server-logs.js';
import * as ssh_settings_tab from './tab-credentials-settings.js';

let comming_soon;

function display_comming_soon() {
    comming_soon = document.querySelectorAll(".temp-disabled");
    comming_soon_render();
    window.addEventListener("resize", function () {
        comming_soon_resize();
    });
}
function comming_soon_render() {
    comming_soon.forEach(function (element) {
        const info = element.parentNode.insertBefore(document.createElement("div"), element.nextSibling);
        info.classList.add("temp-info");
        info.innerHTML = "Coming soon";
        info.style.marginLeft = ((element.getBoundingClientRect().width / 2) - (info.getBoundingClientRect().width / 2)) + "px";
        info.style.marginTop = ((element.getBoundingClientRect().height / 2) * -1 - (info.getBoundingClientRect().height / 2)) + "px";
    });
}
function comming_soon_resize() {
    comming_soon.forEach(function (element) {
        const info = element.nextSibling;
        info.style.marginLeft = ((element.getBoundingClientRect().width / 2) - (info.getBoundingClientRect().width / 2)) + "px";
        info.style.marginTop = ((element.getBoundingClientRect().height / 2) * -1 - (info.getBoundingClientRect().height / 2)) + "px";
    });
}
display_comming_soon();
model_hosting_tab.init();
upload_tab.init();
finetune_tab.init();
access_control_tab.init();
server_log_tab.init();
ssh_settings_tab.init();

const tab_buttons = document.querySelectorAll('.main-tab-button');
const tab_panes = document.querySelectorAll('.main-tab-pane');

tab_buttons.forEach(tab_button => {
  tab_button.addEventListener('click', () => {
    if(tab_button.hasAttribute('disabled')) { return };
    const target_tab = tab_button.dataset.tab;

    tab_buttons.forEach(btn => {
      btn.classList.remove('main-active');
    });

    tab_panes.forEach(pane => {
      if (pane.id === target_tab) {
        pane.classList.add('main-active');
        comming_soon_resize();
      } else {
        pane.classList.remove('main-active');
      }
    });

    tab_button.classList.add('main-active');
  });
});

// remove when schedule will be implemented
const schedule_modal = document.getElementById('finetune-tab-autorun-settings-modal');
schedule_modal.addEventListener('show.bs.modal', function () {
    const elm = document.querySelector('#finetune-tab-autorun-settings-modal .modal-body');
    const info = elm.parentNode.insertBefore(document.createElement("div"), elm);
    elm.style.opacity = 0.2;
    elm.style.pointerEvents = "none";
    elm.style.position = "relative";
    elm.style.zIndex = "0";
    info.classList.add("temp-info-modal");
    info.innerHTML = "Coming soon";
    info.style.marginLeft = '170px';
    info.style.marginTop = '180px';
});

function active_tab_function() {
    const active_tab = document.querySelector('.main-tab-pane.main-active');
    switch (active_tab.id) {
        case 'model-hosting':
            model_hosting_tab.tab_switched_here();
            break;
        case 'upload':
            upload_tab.tab_switched_here();
            break;
        case 'finetune':
            finetune_tab.tab_switched_here();
            break;
        case 'server-logs':
            server_log_tab.tab_switched_here();
            break;
        case 'settings':
            ssh_settings_tab.tab_switched_here();
            break;
        case "access-control":
            break;
    }
}

let refresh_interval = null;

function start_tab_timer() {
    active_tab_function();
    if (refresh_interval) {
        clearInterval(refresh_interval);
    }
    refresh_interval = setInterval(active_tab_function, 2000);
}

start_tab_timer();
