import { O as attr_class, P as ensure_array_like, K as derived } from "../../chunks/index.js";
import { e as escape_html, l as attr } from "../../chunks/attributes.js";
import "@tauri-apps/api/window";
import "@tauri-apps/api/webviewWindow";
import "@tauri-apps/api/core";
import "@tauri-apps/api/event";
function html(value) {
  var html2 = String(value ?? "");
  var open = "<!---->";
  return open + html2 + "<!---->";
}
function filterEntries(items, query) {
  const q = query.trim().toLowerCase();
  if (!q) return items;
  return items.filter((i) => i.content.toLowerCase().includes(q));
}
function escapeHtml(s) {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
}
function highlight(text, query) {
  const q = query.trim().toLowerCase();
  const escaped = escapeHtml(text);
  if (!q) return escaped;
  const idx = escaped.toLowerCase().indexOf(q);
  if (idx < 0) return escaped;
  return escaped.slice(0, idx) + "<mark>" + escaped.slice(idx, idx + q.length) + "</mark>" + escaped.slice(idx + q.length);
}
function TabBar($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let { tab, pinnedCount } = $$props;
    $$renderer2.push(`<div class="tabs svelte-1n51new"><button${attr_class("svelte-1n51new", void 0, { "active": tab === "all" })}>剪贴板</button> `);
    if (pinnedCount > 0) {
      $$renderer2.push("<!--[0-->");
      $$renderer2.push(`<button${attr_class("svelte-1n51new", void 0, { "active": tab === "pinned" })}>固定</button>`);
    } else {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]--></div>`);
  });
}
function EntryRow($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let { entry, selected, query } = $$props;
    const typeLabel = { text: "T", rtf: "R", image: "🖼", file: "🗂", url: "🔗" };
    $$renderer2.push(`<div${attr_class("row svelte-1ia6pls", void 0, { "selected": selected })} role="button" tabindex="-1"><span class="ico svelte-1ia6pls">${escape_html(typeLabel[entry.type])}</span> <span class="txt svelte-1ia6pls">${html(highlight(entry.content, query))}</span> `);
    if (entry.pinned) {
      $$renderer2.push("<!--[0-->");
      $$renderer2.push(`<span class="pin svelte-1ia6pls">📌</span>`);
    } else {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]--></div>`);
  });
}
function _page($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let entries = [];
    let query = "";
    let tab = "all";
    let selected = 0;
    let visibleEntries = derived(() => filterEntries(entries, query));
    $$renderer2.push(`<div class="pop"><div class="search svelte-1uha8ag"><input${attr("value", query)} placeholder="搜索剪贴板…"/> <button class="gear svelte-1uha8ag" title="设置">⚙︎</button></div> `);
    TabBar($$renderer2, {
      tab,
      pinnedCount: entries.filter((e) => e.pinned).length
    });
    $$renderer2.push(`<!----> <div class="list">`);
    if (visibleEntries().length === 0) {
      $$renderer2.push("<!--[0-->");
      $$renderer2.push(`<div class="empty">${escape_html("暂无剪贴板记录")}</div>`);
    } else {
      $$renderer2.push("<!--[-1-->");
      $$renderer2.push(`<!--[-->`);
      const each_array = ensure_array_like(visibleEntries());
      for (let i = 0, $$length = each_array.length; i < $$length; i++) {
        let entry = each_array[i];
        EntryRow($$renderer2, {
          entry,
          selected: i === selected,
          query
        });
      }
      $$renderer2.push(`<!--]-->`);
    }
    $$renderer2.push(`<!--]--></div> <div class="foot"><span>↑↓ 选择 · ⏎ 粘贴</span> <span>⌘P 固定 · ⌫ 删除 · Esc 关闭</span></div></div>`);
  });
}
export {
  _page as default
};
