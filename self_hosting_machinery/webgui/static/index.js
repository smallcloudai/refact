
const req = await fetch('list-plugins');
const plugins = await req.json();

// show navigation bar immediately, import later
plugins_to_top_nav_bar(plugins);

// this might take some time: load all modules
const imported_plugins = [];
const inits_working = [];
for (const p of plugins) {
    const mod = await import("./tab-" + p.tab + ".js");
    imported_plugins.push(mod);
    p.mod = mod;
    inits_working.push(mod.init());
}
for (const p of inits_working) {
    await p;
}

function active_tab_switched() {
    const active_tab = document.querySelector('.main-tab-pane.main-active');
    for (const plugin of plugins) {
        if (active_tab.id !== plugin.tab) {
            plugin.mod.tab_switched_away();
        }
    }
    for (const plugin of plugins) {
        if (active_tab.id === plugin.tab) {
            plugin.mod.tab_switched_here();
            break;
        }
    }
}

function every_couple_of_seconds() {
    const active_tab = document.querySelector('.main-tab-pane.main-active');
    for (const plugin of plugins) {
        if (active_tab.id === plugin.tab) {
            plugin.mod.tab_update_each_couple_of_seconds();
            break;
        }
    }
}

let refresh_interval = null;

function start_tab_timer() {
    active_tab_switched();
    if (refresh_interval) {
        clearInterval(refresh_interval);
    }
    refresh_interval = setInterval(every_couple_of_seconds, 2000);
}

start_tab_timer();


function plugins_to_top_nav_bar(plugins)
{
    // show immediately, import later
    const template_ltr = Handlebars.compile(document.getElementById('nav-template-ltr').innerHTML);
    const html = template_ltr({ "items": plugins });
    document.getElementById('nav-container').innerHTML = html;

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
            start_tab_timer();
        });
    });
}


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
    document.querySelector('#finetune-tab-autorun-settings-modal .modal-footer').style.display = 'none';
});

