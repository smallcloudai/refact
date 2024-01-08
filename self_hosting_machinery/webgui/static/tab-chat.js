export async function init() {
  const elem = document.querySelector("#chat");
  const flex = document.createElement("div");
  flex.style.display = "flex";
  const iframe = document.createElement("iframe");
  iframe.src = "tab-chat.html";
  iframe.className = "container-lg pane";
  // TODO: calculate hight for multiple displays
  iframe.style.height = "100%";
  iframe.style.minHeight = "470px";
  flex.appendChild(iframe);
  elem.appendChild(flex);
}

export function tab_switched_here() {

}

export function tab_switched_away() {

}

export function tab_update_each_couple_of_seconds() {}
