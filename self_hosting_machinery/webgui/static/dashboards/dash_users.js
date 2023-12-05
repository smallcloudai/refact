import {fetch_plots_data, fetch_teams_dashboard_data} from "./utils.js";
import {render_plot_with_buttons, barplot_completions_users} from "./plots.js";


let html = `
<div class="row" style="width: 100%">
    <div class="col-md-3">
        <div id="dusers-teams-tables" style="margin-bottom: 5px"></div>
    </div>
    <div class="col-md-9">
        <div id="dusers-barplots-completions-users"></div>
        <div id="dusers-barplots-workdays-users" style="margin-top: 30px"></div>
    </div>
    
</div>
`


function create_table(team_name, users) {
    const table = document.createElement("table");
    table.className = "table table-striped dusers-teams-table";

    const table_head = document.createElement("thead");
    const head_row = document.createElement("tr");
    head_row.innerHTML = `
        <th><input type="checkbox" class="dusers-teams-table-select-all" data-target="${team_name}"></th>
        <th>${team_name}</th>
    `;
    table_head.appendChild(head_row);
    table.appendChild(table_head);

    const table_body = document.createElement("tbody");

    users.forEach(user => {
        const row = document.createElement("tr");
        row.innerHTML = `
            <td><input type="checkbox"></td>
            <td>${user}</td>
        `;
        table_body.appendChild(row);
    });

    table.appendChild(table_body);
    return table;
}


async function render_plots(data) {
    document.querySelector('#dusers-barplots-completions-users').innerHTML = "";
    await render_plot_with_buttons('dusers-barplots-completions-users', data['barplot_completions_users'], barplot_completions_users, "dash-plot-sm");
}

async function create_team_tables(data) {
    const team_tables_container = document.getElementById("dusers-teams-tables");

    for (const team in data) {
        if (data.hasOwnProperty(team)) {
            const team_table = create_table(team, data[team]["users"]);
            const team_div = document.createElement("div");
            team_div.className = `dusers-teams-table-col dusers-teams-table-col-${team}`

            team_div.appendChild(team_table);
            team_tables_container.appendChild(team_div);
        }
    }
    let teams_tables_select_all = document.querySelectorAll(".dusers-teams-table-select-all");

    teams_tables_select_all.forEach(el0 => {
        el0.addEventListener("click", () => {
            const target_team = el0.dataset.target;
            const team_table = document.querySelector(`.dusers-teams-table-col-${target_team} table`);
            team_table.querySelectorAll("input[type=checkbox]").forEach(el => {
                if (!el.disabled) {
                    el.checked = el0.checked;
                }
            });
        });
    });

    // action on checkbox change
    let teams_tables_checkboxes = document.querySelectorAll(".dusers-teams-table-col input[type=checkbox]");
    teams_tables_checkboxes.forEach(el => {
        el.addEventListener("change", async () => {
            let users_checked = [];
            // fill users_checked: for each checkbox: if it's in td, get first field and push to users_checked
            teams_tables_checkboxes.forEach(el0 => {
                if (el0.parentElement.tagName === "TD") {
                    if (el0.checked) {
                        users_checked.push(el0.parentElement.nextElementSibling.innerText);
                    }
                }
            });
            if (users_checked.length !== 0) {
                let data = await fetch_teams_dashboard_data({"users_selected": users_checked});
                console.log("data", data);
                await render_plots(data);
            }
        });
    });
    // click on first team table checkbox
    teams_tables_checkboxes[0].click();
}


export async function init(insert_in_el,) {
    insert_in_el.innerHTML = html;

    let plots_teams_data = await fetch_plots_data('dash-teams');
    await create_team_tables(plots_teams_data["teams_data"]);
}


export function switch_away(el) {
    el.innerHTML = "";
}
