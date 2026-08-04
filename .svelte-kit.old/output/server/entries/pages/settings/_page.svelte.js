import { e as escape_html, l as attr } from "../../../chunks/attributes.js";
import "@tauri-apps/api/core";
import "@tauri-apps/api/event";
const prerender = true;
function _page($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let settings = { history_limit: 15, shortcut: "Command+Shift+V" };
    let status = "";
    $$renderer2.push(`<div class="settings svelte-1i19ct2"><h2 class="svelte-1i19ct2">ClipboardX 设置</h2> <section class="svelte-1i19ct2"><h3 class="svelte-1i19ct2">常规</h3> <label class="svelte-1i19ct2">历史记录上限（${escape_html(settings.history_limit)} 条，最大 100） <input type="range" min="1" max="100"${attr("value", settings.history_limit)} class="svelte-1i19ct2"/></label> <p class="hint svelte-1i19ct2">超过上限自动删除最旧的未固定条目。</p></section> <section class="svelte-1i19ct2"><h3 class="svelte-1i19ct2">快捷键</h3> <button class="keybtn svelte-1i19ct2">${escape_html(settings.shortcut)}</button></section> <p class="status svelte-1i19ct2">${escape_html(status)}</p></div>`);
  });
}
export {
  _page as default,
  prerender
};
