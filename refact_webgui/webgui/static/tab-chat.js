export async function init() {
}


export function tab_switched_here() {
    const tab_buttons = document.querySelectorAll('.main-tab-button');
    for (let i = 0; i < tab_buttons.length; i++) {
        if (tab_buttons[i].id === "default") {
            tab_buttons[i].click();
        }
    }
    window.open('/chat', '_blank');
}


export function tab_switched_away() {
}


export function tab_update_each_couple_of_seconds() {
}
