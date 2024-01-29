import { general_error } from './error.js';


export async function init(general_error) {
    let req = await fetch('/tab-version.html');
    document.querySelector('#version').innerHTML = await req.text();
}


export function tab_settings_integrations_get() {
    fetch("/tab-version-get")
        .then(function(response) {
            return response.json();
        })
        .then(function(data) {
            const tab_version = document.getElementById("refact-version")
            var version_table_data = "";
            ["a", "b", "c"].forEach((key) => {
                version_table_data += `
                    <tr>
                    <td><label class="refact-item-name">${key}</label></td>
                    </tr>`;
            });
            tab_version.innerHTML = `<div><table>${version_table_data}</table></div>`;
        });
}

export function tab_switched_here() {
    refact_version_get();
}


export function tab_switched_away() {
}


export function tab_update_each_couple_of_seconds() {
}
