import "./styles.css";

type CollectionSummary = { id: string; count: number };
type CatalogSummary = { id: string; label: string; collections: CollectionSummary[] };
type JsonObject = Record<string, unknown>;

const app = document.querySelector<HTMLDivElement>("#app")!;
let username = localStorage.getItem("admin_user") ?? "admin";
let password = "";
let catalogs: CatalogSummary[] = [];
let catalogData: JsonObject = {};
let activeCatalog = "";
let activeCollection = "";
let query = "";
let selected: unknown;

const escapeHtml = (value: unknown) => String(value ?? "").replace(/[&<>"']/g, (character) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[character]!));
const authHeaders = () => ({ Authorization: `Basic ${btoa(`${username}:${password}`)}` });
const request = (path: string) => fetch(path, { headers: authHeaders() });

function displayValue(value: unknown) {
  if (value === null) return "null";
  if (typeof value === "object") return Array.isArray(value) ? `[${value.length} values]` : `{${Object.keys(value as JsonObject).length} fields}`;
  return String(value);
}

function rowTitle(row: unknown, index: number) {
  if (!row || typeof row !== "object") return `Value ${index + 1}`;
  const object = row as JsonObject;
  for (const key of ["displayName", "name", "id", "catalogId", "buildingId", "monsterId", "materialId", "key", "index"]) {
    if (object[key] !== undefined) return String(object[key]);
  }
  return `Object ${index + 1}`;
}

function currentRows() {
  const value = activeCollection.split(".").reduce<unknown>((current, segment) => current && typeof current === "object" ? (current as JsonObject)[segment] : undefined, catalogData);
  const rows = Array.isArray(value) ? value : value === undefined ? [] : [value];
  const needle = query.trim().toLowerCase();
  return needle ? rows.filter((row) => JSON.stringify(row).toLowerCase().includes(needle)) : rows;
}

function render() {
  const summary = catalogs.find((catalog) => catalog.id === activeCatalog);
  const rows = currentRows();
  app.innerHTML = `<div class="min-h-screen lg:flex">
    <aside class="border-b border-slate-800 bg-[#07111f] text-slate-300 lg:min-h-screen lg:w-72 lg:border-b-0 lg:border-r">
      <div class="flex h-20 items-center gap-3 border-b border-white/10 px-6"><div class="grid h-9 w-9 place-items-center rounded-xl bg-cyan-400 font-black text-slate-950">EH</div><div><p class="text-sm font-semibold text-white">Evil Hunter</p><p class="text-[11px] text-slate-500">Catalog object browser</p></div></div>
      <nav class="flex gap-2 overflow-x-auto p-4 lg:block lg:space-y-1">${catalogs.map((catalog) => `<button data-catalog="${escapeHtml(catalog.id)}" class="nav-item shrink-0 ${catalog.id === activeCatalog ? "active" : ""}"><span>${escapeHtml(catalog.label)}</span><span class="ml-auto text-[10px] opacity-60">${catalog.collections.reduce((total, collection) => total + collection.count, 0)}</span></button>`).join("")}</nav>
    </aside>
    <main class="min-w-0 flex-1"><header class="border-b border-slate-200 bg-white px-5 py-5 sm:px-8"><p class="text-xs font-semibold uppercase tracking-[.16em] text-cyan-600">${escapeHtml(summary?.label ?? "Admin")}</p><div class="mt-1 flex flex-wrap items-end justify-between gap-3"><h1 class="text-2xl font-bold text-slate-900">Runtime objects</h1><span class="rounded-full bg-emerald-50 px-3 py-1.5 text-xs font-semibold text-emerald-700">Authoritative catalog · read only</span></div></header>
      <div class="p-5 sm:p-8"><div class="mb-5 flex flex-col gap-3 xl:flex-row"><select id="collection" class="rounded-lg border border-slate-200 bg-white px-3 py-2.5 text-sm">${summary?.collections.map((collection) => `<option value="${escapeHtml(collection.id)}" ${collection.id === activeCollection ? "selected" : ""}>${escapeHtml(collection.id)} (${collection.count})</option>`).join("") ?? ""}</select><input id="search" value="${escapeHtml(query)}" placeholder="Search every field in this collection..." class="min-w-0 flex-1 rounded-lg border border-slate-200 bg-white px-4 py-2.5 text-sm outline-none focus:ring-2 focus:ring-cyan-500"><div class="rounded-lg border border-slate-200 bg-white px-4 py-2.5 text-sm text-slate-500">${rows.length} matching objects</div></div>
        <div class="grid gap-5 xl:grid-cols-[minmax(0,1fr)_minmax(360px,0.7fr)]"><section class="overflow-hidden rounded-2xl border border-slate-200 bg-white shadow-sm"><div class="max-h-[70vh] overflow-auto"><table class="w-full text-left text-sm"><thead class="sticky top-0 bg-slate-50 text-[11px] uppercase tracking-wider text-slate-500"><tr><th class="px-5 py-3">#</th><th class="px-5 py-3">Object</th><th class="px-5 py-3">Preview</th><th class="px-5 py-3 text-right">Fields</th></tr></thead><tbody class="divide-y divide-slate-100">${rows.map((row, index) => `<tr data-row="${index}" class="cursor-pointer hover:bg-cyan-50/50"><td class="px-5 py-3 font-mono text-xs text-slate-400">${index + 1}</td><td class="px-5 py-3 font-semibold text-slate-800">${escapeHtml(rowTitle(row, index))}</td><td class="max-w-md truncate px-5 py-3 text-slate-500">${escapeHtml(displayValue(row))}</td><td class="px-5 py-3 text-right text-slate-500">${row && typeof row === "object" ? Object.keys(row).length : 1}</td></tr>`).join("") || '<tr><td colspan="4" class="px-5 py-12 text-center text-slate-500">No matching objects.</td></tr>'}</tbody></table></div></section>
          <section class="overflow-hidden rounded-2xl border border-slate-200 bg-[#07111f] shadow-sm"><div class="border-b border-white/10 px-5 py-4 text-sm font-semibold text-white">Complete object JSON</div><pre class="max-h-[70vh] overflow-auto whitespace-pre-wrap break-words p-5 text-xs leading-6 text-cyan-100">${escapeHtml(selected === undefined ? "Select an object to inspect every field." : JSON.stringify(selected, null, 2))}</pre></section></div>
      </div></main></div>`;
  bind();
}

function bind() {
  document.querySelectorAll<HTMLButtonElement>("[data-catalog]").forEach((button) => button.addEventListener("click", () => loadCatalog(button.dataset.catalog!)));
  document.querySelector<HTMLSelectElement>("#collection")?.addEventListener("change", (event) => { activeCollection = (event.target as HTMLSelectElement).value; query = ""; selected = undefined; render(); });
  document.querySelector<HTMLInputElement>("#search")?.addEventListener("input", (event) => { query = (event.target as HTMLInputElement).value; selected = undefined; render(); const input = document.querySelector<HTMLInputElement>("#search"); input?.focus(); input?.setSelectionRange(query.length, query.length); });
  document.querySelectorAll<HTMLTableRowElement>("[data-row]").forEach((row) => row.addEventListener("click", () => { selected = currentRows()[Number(row.dataset.row)]; render(); }));
}

async function loadCatalog(catalogId: string) {
  const response = await request(`/admin/catalogs/${encodeURIComponent(catalogId)}`);
  if (!response.ok) return showLogin();
  activeCatalog = catalogId;
  catalogData = await response.json();
  activeCollection = catalogs.find((catalog) => catalog.id === catalogId)?.collections[0]?.id ?? "";
  query = "";
  selected = undefined;
  render();
}

function showLogin() {
  app.innerHTML = `<div class="grid min-h-screen place-items-center bg-[#07111f] p-4"><form class="w-full max-w-sm rounded-2xl bg-white p-7 shadow-2xl"><div class="mb-4 grid h-10 w-10 place-items-center rounded-xl bg-cyan-400 font-black text-slate-950">EH</div><h1 class="text-xl font-bold text-slate-900">Admin catalog access</h1><p class="mt-1 text-sm text-slate-500">Sign in to inspect migrated runtime objects.</p><label class="mt-6 block text-xs font-semibold text-slate-600">Username<input name="username" autocomplete="username" required value="${escapeHtml(username)}" class="mt-2 w-full rounded-lg border border-slate-200 px-3 py-2.5 text-sm"></label><label class="mt-4 block text-xs font-semibold text-slate-600">Password<input name="password" type="password" autocomplete="current-password" required class="mt-2 w-full rounded-lg border border-slate-200 px-3 py-2.5 text-sm"></label><p class="error mt-3 hidden text-xs font-semibold text-red-600">Invalid credentials or server unavailable.</p><button class="mt-6 w-full rounded-lg bg-slate-900 px-4 py-2.5 text-sm font-semibold text-white">Continue</button></form></div>`;
  app.querySelector("form")!.addEventListener("submit", async (event) => { event.preventDefault(); const data = new FormData(event.currentTarget as HTMLFormElement); username = String(data.get("username")); password = String(data.get("password")); localStorage.setItem("admin_user", username); await bootstrap(); });
}

async function bootstrap() {
  try {
    const response = await request("/admin/catalogs");
    if (!response.ok) throw new Error("unauthorized");
    catalogs = (await response.json()).catalogs;
    if (!catalogs.length) throw new Error("empty catalogs");
    await loadCatalog(catalogs[0].id);
  } catch {
    password = "";
    showLogin();
    app.querySelector(".error")?.classList.remove("hidden");
  }
}

showLogin();
