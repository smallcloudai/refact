import {fetch_plots_data, fill_table_with_data} from "./utils.js";
import {barplot_completions, barplot_rh, barplot_users, render_plot_with_buttons} from "./plots.js";


let html = `
<div id="dprime-table-lang-comp-stats-wrapper" style="width: 1000px; height: auto; margin-top: 10px; margin-bottom: 20px">
<div class="row" style="margin-bottom: 10px">
    <div id="dprime-table-lang-comp-stats-title" class="col-md-6"><h5></h5></div>
    <div id="dprime-table-lang-comp-stats-btns" class="col-md-6" style="text-align: right"></div>
    <div id="dprime-table-wrapper"></div>
</div>
</div>
<div id="dprime-barplots-rh" style="margin-top: 40px">
</div>
<div id="dprime-barplots-completions" style="margin-top: 40px">
</div>
<div id="dprime-barplots-users" style="margin-top: 40px">
</div>
`

function render_table_lang_comp_stats(table_wrapper, title_el, data) {
    title_el.innerText = data['title'];

    let table = document.createElement('table');
    table.className = "table table-striped";
    table.style.width = "100%";
    table.style.height = "auto";
    table.style.marginTop = "10px";
    table_wrapper.innerHTML = "";
    table_wrapper.appendChild(table);
    fill_table_with_data(table, data);
}


function render_table_lang_comp_stats_with_buttons(wrap_id, table_lang_comp_stats_data) {
    const btns_data = table_lang_comp_stats_data['btns_data'];
    let title_el = document.querySelector(`#dprime-table-lang-comp-stats-title h5`);

    let btn_group = document.querySelector(`#dprime-table-lang-comp-stats-btns`);

    let table_wrapper = document.querySelector(`#dprime-table-wrapper`);
    let grid;
    for (const [_, btn_text] of Object.entries(btns_data['btns_text'])) {
        let btn_id = `${wrap_id}-btn-${btn_text}`;

        let btn = document.createElement('button');
        btn.type = "button";
        btn.className = "btn btn-outline-secondary btn-sm dash-plot-btn";
        btn.id = btn_id;
        btn.innerText = btn_text;
        btn.setAttribute("data-target", btn_text);
        btn_group.appendChild(btn);

        if (btn_text === btns_data['default']) {
            btn.classList.add('active');
            render_table_lang_comp_stats(table_wrapper, title_el, table_lang_comp_stats_data[btn_text]);
        }
    }

    let btn_group_btn = document.querySelectorAll(`#dprime-table-lang-comp-stats-btns button`);

    btn_group_btn.forEach((el) => {
        el.addEventListener('click', (event) => {
            btn_group_btn.forEach((el0) => {
                el0.classList.remove('active');
            });
            event.target.classList.add('active');
            for (const [_, btn_text] of Object.entries(btns_data['btns_text'])) {
                if (btn_text === event.target.dataset.target) {
                    render_table_lang_comp_stats(table_wrapper, title_el, table_lang_comp_stats_data[btn_text], grid);
                }
            }
        });
    });

}


export async function init(insert_in_el,) {
    insert_in_el.innerHTML = html;

    let plots_data = await fetch_plots_data('dash-prime');

    render_table_lang_comp_stats_with_buttons('dprime-table-lang-comp-stats-wrapper', plots_data['table_lang_comp_stats']);

    render_plot_with_buttons('dprime-barplots-rh', plots_data['barplot_rh'], barplot_rh);
    render_plot_with_buttons('dprime-barplots-completions', plots_data['barplot_completions'], barplot_completions);
    render_plot_with_buttons('dprime-barplots-users', plots_data['barplot_users'], barplot_users);

    console.info('table_lang_comp_stats', plots_data['table_lang_comp_stats']);
}


export function switch_away(el) {
    el.innerHTML = "";
}
