import "./styles.css";

type JsonRow = Record<string, unknown>;
type PageResponse = { data: JsonRow[]; page: number; pageSize: number; total: number; totalPages: number };
type WikiTier = { tier: number; requiredItemLevel: number; minimumValue: number; maximumValue: number; valueBasis: string };
type WikiEntry = { id: string; sourceId: number | null; nameEn: string; nameVi: string; origin: string; slot: string; family: string; exclusiveGroup: string; weight: number; minimumDifficulty: number; maximumDifficulty: number; evidence: string; active: boolean; tiers: WikiTier[]; releaseId: string };
type WikiResponse = { releaseId: string | null; tierPolicy: string; entries: WikiEntry[] };
type Overview = { players: number; hunters: number; items: number; releases: number; tickRate: number; protocolVersion: number; durableSchemaVersion: number };
type Column = { key: string; label: string; format?: (value: unknown, row: JsonRow) => string };
type PageConfig = { label: string; section: string; endpoint: string; description: string; columns: Column[] };

const configs: Record<string, PageConfig> = {
  items: { label: "Items", section: "Game content", endpoint: "items", description: "Economy resources and localized item definitions.", columns: cols(["name", "Name"], ["id", "Item ID"], ["category", "Type"], ["stackLimit", "Stack"], ["sellValue", "Sell value"], ["status", "Release status"]) },
  buildings: { label: "Buildings", section: "Game content", endpoint: "buildings", description: "Authoritative building definitions and placement bounds.", columns: cols(["name", "Name"], ["id", "Building ID"], ["category", "Category"], ["gridWidth", "Width"], ["gridHeight", "Height"], ["maxInstances", "Max"], ["constructible", "Constructible"]) },
  monsters: { label: "Monsters", section: "Game content", endpoint: "monsters", description: "Monster stats loaded by the authoritative simulation.", columns: cols(["id", "Source ID"], ["monsterType", "Type"], ["level", "Level"], ["hp", "HP"], ["damage", "Damage"], ["armor", "Armor"], ["experience", "EXP"], ["gold", "Gold"]) },
  gear: { label: "Gear", section: "Game content", endpoint: "gear", description: "Weapons, armor and equipment catalog objects.", columns: cols(["name", "Name"], ["id", "Gear ID"], ["kind", "Kind"], ["job", "Job"], ["itemLevel", "Item level"], ["difficulty", "Difficulty"], ["visibility", "Visibility"]) },
  rebuildWeapons: { label: "Weapon bases", section: "Weapon system", endpoint: "rebuild-weapons", description: "Forty bilingual weapon bases with a fixed implicit Attack Damage range and legacy package icons.", columns: cols(["inventoryIcon", "Icon"], ["nameEn", "English"], ["nameVi", "Vietnamese"], ["className", "Class"], ["difficulty", "Difficulty"], ["unlockLevel", "Unlock"], ["levelCap", "Level cap"], ["attackDamageLine", "Implicit damage"], ["assetState", "Assets"]) },
  affixes: { label: "Modifiers", section: "Weapon system", endpoint: "affixes", description: "The complete mined property catalog plus rebuild-designed weapon affixes. Pool assignments remain visibly separated from package evidence.", columns: cols(["sourceId", "Source ID"], ["nameEn", "English"], ["nameVi", "Vietnamese"], ["origin", "Origin"], ["kind", "Kind"], ["slot", "Slot"], ["family", "Family"], ["exclusiveGroup", "Exclusive group"], ["generationState", "Pool state"]) },
  affixTiers: { label: "Modifier tiers", section: "Weapon system", endpoint: "affix-tiers", description: "Eight deterministic value bands for every active weapon prefix and suffix family.", columns: cols(["nameEn", "Modifier"], ["slot", "Slot"], ["family", "Family"], ["difficulty", "Difficulty"], ["minimumItemLevel", "Min level"], ["maximumItemLevel", "Max level"], ["minimumValue", "Roll", rollRange], ["valueBasis", "Value basis"]) },
  affixPools: { label: "Weapon pools", section: "Weapon system", endpoint: "affix-pools", description: "Active weighted prefix and suffix pools with duplicate-prevention groups and difficulty boundaries.", columns: cols(["nameEn", "Modifier"], ["nameVi", "Vietnamese"], ["slot", "Slot"], ["family", "Family"], ["exclusiveGroup", "Exclusive group"], ["weight", "Weight"], ["minimumDifficulty", "Min difficulty"], ["maximumDifficulty", "Max difficulty"], ["active", "Active"]) },
  wiki: { label: "Weapon Wiki", section: "Weapon system", endpoint: "wiki/weapon-modifiers", description: "A readable reference for active weapon modifiers, their weights, level bands and roll ranges.", columns: [] },
  virtues: { label: "Virtue effects", section: "Weapon system", endpoint: "virtues", description: "The five package-confirmed Virtue effects, kept outside prefix and suffix slot budgets.", columns: cols(["sourceId", "Source ID"], ["nameEn", "English"], ["nameVi", "Vietnamese"], ["thresholds", "Thresholds", jsonValue], ["secondaryValue", "Secondary"], ["descriptionEn", "Effect"], ["evidence", "Evidence"]) },
  collectionSets: { label: "Collection sets", section: "Weapon system", endpoint: "collection-sets", description: "All 61 mined collection-set definitions. Raw option semantics remain unresolved and are not active gameplay rules.", columns: cols(["sourceId", "Source ID"], ["nameEn", "English"], ["nameVi", "Vietnamese"], ["itemIds", "Item IDs", jsonValue], ["optionType", "Option type"], ["optionValue", "Option value"], ["effectState", "Effect state"]) },
  consumables: { label: "Consumables", section: "Game content", endpoint: "consumables", description: "Consumable definitions, levels and cooldown boundaries.", columns: cols(["id", "Index"], ["type", "Type"], ["maxLevel", "Max level"], ["cooldownMs", "Cooldown (ms)"], ["releaseId", "Release"]) },
  hunters: { label: "Hunters", section: "Live data", endpoint: "hunters", description: "Persisted Hunter roster, progression and current state.", columns: cols(["name", "Hunter"], ["hunterId", "ID"], ["className", "Class"], ["rarityName", "Rarity"], ["level", "Level"], ["currentHp", "HP", hp], ["gold", "Gold"], ["actionState", "State"]) },
  players: { label: "Players", section: "Live data", endpoint: "players", description: "Player aggregates, revisions and authoritative town value.", columns: cols(["id", "Player ID"], ["revision", "Revision"], ["hunterCount", "Hunters"], ["townGold", "Town gold"], ["updatedAt", "Updated"]) },
  releases: { label: "Content releases", section: "Operations", endpoint: "releases", description: "Immutable gameplay and Hunter content releases.", columns: cols(["id", "Release ID"], ["kind", "Kind"], ["status", "Status"], ["createdAt", "Created"]) },
  audit: { label: "Audit log", section: "Operations", endpoint: "audit", description: "Authoritative command and reward ledger activity.", columns: cols(["createdAt", "Time"], ["kind", "Kind"], ["action", "Action"], ["playerId", "Player ID"]) },
};

function cols(...entries: ([string, string] | [string, string, (value: unknown, row: JsonRow) => string])[]): Column[] { return entries.map(([key, label, format]) => ({ key, label, format })); }
function hp(value: unknown, row: JsonRow) { return `${number(value)} / ${number(row.maxHp)}`; }
function jsonValue(value: unknown) { return value == null ? "-" : JSON.stringify(value); }
function rollRange(value: unknown, row: JsonRow) { return `${number(value)}–${number(row.maximumValue)}`; }
function number(value: unknown) { return typeof value === "number" ? value.toLocaleString() : value == null ? "-" : String(value); }
function escape(value: unknown) { return String(value ?? "-").replace(/[&<>'"]/g, (char) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", "'": "&#39;", '"': "&quot;" })[char]!); }

const app = document.querySelector<HTMLDivElement>("#app")!;
let username = localStorage.getItem("admin_user") ?? "admin";
let password = "";
let route = routeFromHash();
let currentPage = 1;
let pageSize = 25;
let search = "";
let loading = false;
let pageData: PageResponse | null = null;
let overview: Overview | null = null;
let wikiData: WikiResponse | null = null;
let wikiSlot = "all";
let wikiTier = "all";

window.addEventListener("hashchange", () => { route = routeFromHash(); currentPage = 1; search = ""; pageData = null; overview = null; wikiData = null; void load(); });

function routeFromHash() { const value = location.hash.replace(/^#\/?/, ""); return value === "overview" || configs[value] ? value : "overview"; }
function authHeader() { return { Authorization: `Basic ${btoa(`${username}:${password}`)}` }; }

async function api<T>(path: string): Promise<T> {
  const response = await fetch(`/admin/${path}`, { headers: authHeader() });
  if (response.status === 401) { password = ""; showLogin(); throw new Error("unauthorized"); }
  if (!response.ok) throw new Error(`request_failed_${response.status}`);
  return response.json() as Promise<T>;
}

async function load() {
  if (!password) { render(); showLogin(); return; }
  loading = true; render();
  try {
    if (route === "overview") overview = await api<Overview>("overview");
    else if (route === "wiki") wikiData = await api<WikiResponse>(`${configs[route].endpoint}?search=${encodeURIComponent(search)}`);
    else pageData = await api<PageResponse>(`${configs[route].endpoint}?page=${currentPage}&pageSize=${pageSize}&search=${encodeURIComponent(search)}`);
  } catch (error) { if ((error as Error).message !== "unauthorized") showError(); }
  finally { loading = false; render(); }
}

function render() {
  const title = route === "overview" ? "Overview" : configs[route].label;
  const description = route === "overview" ? "Authoritative runtime and content health at a glance." : configs[route].description;
  app.innerHTML = `<div class="flex min-h-screen bg-[#f3f6fa]">
    <aside class="hidden w-72 shrink-0 bg-[#08121f] text-slate-300 lg:flex lg:flex-col">
      <div class="flex h-20 items-center gap-3 border-b border-white/10 px-6"><div class="grid h-10 w-10 place-items-center rounded-xl bg-cyan-400 text-sm font-black text-slate-950">EH</div><div><p class="font-bold text-white">Evil Hunter</p><p class="text-[11px] text-slate-500">Operations console</p></div></div>
      <nav class="flex-1 overflow-y-auto px-4 py-6">${navigation()}</nav>
      <div class="border-t border-white/10 p-4"><div class="flex items-center gap-3 rounded-xl bg-white/5 p-3"><div class="grid h-9 w-9 place-items-center rounded-full bg-cyan-400/15 text-xs font-bold text-cyan-300">AD</div><div class="min-w-0"><p class="truncate text-sm font-semibold text-white">${escape(username)}</p><p class="text-[11px] text-slate-500">Basic Auth session</p></div><button id="logout" class="ml-auto text-xs text-slate-500 hover:text-white">Exit</button></div></div>
    </aside>
    <main class="min-w-0 flex-1"><header class="flex min-h-20 items-center justify-between border-b border-slate-200 bg-white px-5 py-4 sm:px-8"><div><p class="text-[11px] font-bold uppercase tracking-[.18em] text-cyan-600">${route === "overview" ? "Command center" : configs[route].section}</p><h1 class="mt-1 text-2xl font-bold tracking-tight text-slate-950">${title}</h1></div><div class="flex items-center gap-3"><span class="hidden items-center gap-2 rounded-full bg-emerald-50 px-3 py-1.5 text-xs font-semibold text-emerald-700 sm:flex"><span class="h-1.5 w-1.5 rounded-full bg-emerald-500"></span>Authoritative server</span></div></header>
      <div class="mx-auto max-w-[1500px] p-5 sm:p-8"><div class="mb-6"><p class="max-w-3xl text-sm text-slate-500">${description}</p></div>${loading ? skeleton() : route === "overview" ? overviewView() : route === "wiki" ? wikiView() : tableView()}</div>
    </main></div>`;
  bind();
}

function navigation() {
  const groups: Record<string, [string, string][]> = {
    "Command center": [["overview", "Overview"]],
    "Game content": Object.entries(configs).filter(([, c]) => c.section === "Game content").map(([key, c]) => [key, c.label]),
    "Weapon system": Object.entries(configs).filter(([, c]) => c.section === "Weapon system").map(([key, c]) => [key, c.label]),
    "Live data": Object.entries(configs).filter(([, c]) => c.section === "Live data").map(([key, c]) => [key, c.label]),
    "Operations": Object.entries(configs).filter(([, c]) => c.section === "Operations").map(([key, c]) => [key, c.label]),
  };
  return Object.entries(groups).map(([group, pages]) => `<div class="mb-7"><p class="mb-2 px-3 text-[10px] font-bold uppercase tracking-[.18em] text-slate-600">${group}</p><div class="space-y-1">${pages.map(([key, label]) => `<a href="#/${key}" class="nav-item ${route === key ? "active" : ""}"><span class="h-1.5 w-1.5 rounded-full ${route === key ? "bg-cyan-300" : "bg-slate-700"}"></span>${label}</a>`).join("")}</div></div>`).join("");
}

function overviewView() {
  if (!overview) return empty("Overview data is unavailable.");
  const cards = [["Players", overview.players], ["Hunters", overview.hunters], ["Catalog items", overview.items], ["Content releases", overview.releases]];
  return `<div class="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">${cards.map(([label, value]) => `<div class="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm"><p class="text-xs font-semibold text-slate-500">${label}</p><p class="mt-3 text-3xl font-bold text-slate-950">${number(value)}</p></div>`).join("")}</div><div class="mt-6 grid gap-5 lg:grid-cols-3"><div class="rounded-2xl border border-slate-200 bg-white p-6 shadow-sm lg:col-span-2"><h2 class="font-bold text-slate-900">Runtime contract</h2><div class="mt-5 grid gap-4 sm:grid-cols-3">${metric("Tick rate", `${overview.tickRate}/s`)}${metric("Protocol", `v${overview.protocolVersion}`)}${metric("Durable schema", `v${overview.durableSchemaVersion}`)}</div></div><div class="rounded-2xl bg-[#0b1727] p-6 text-white shadow-sm"><p class="text-xs font-bold uppercase tracking-[.16em] text-cyan-300">Safety boundary</p><h2 class="mt-3 text-lg font-bold">Read-only catalog view</h2><p class="mt-2 text-sm leading-6 text-slate-400">Active content remains immutable. Mutations will only target validated draft releases.</p></div></div>`;
}
function metric(label: string, value: string) { return `<div class="rounded-xl bg-slate-50 p-4"><p class="text-xs text-slate-500">${label}</p><p class="mt-1 font-mono text-lg font-bold text-slate-900">${value}</p></div>`; }

function tableView() {
  const config = configs[route];
  const data = pageData;
  if (!data) return empty("No data returned by the server.");
  return `<section class="overflow-hidden rounded-2xl border border-slate-200 bg-white shadow-sm"><div class="flex flex-col gap-4 border-b border-slate-200 p-5 sm:flex-row sm:items-center sm:justify-between"><div><h2 class="font-bold text-slate-900">${config.label} catalog</h2><p class="mt-1 text-xs text-slate-500">${number(data.total)} objects found</p></div><div class="flex gap-2"><button id="refresh" class="rounded-lg border border-slate-200 px-3 py-2 text-sm font-semibold text-slate-600 hover:bg-slate-50">Refresh</button><button class="cursor-not-allowed rounded-lg bg-slate-200 px-3 py-2 text-sm font-semibold text-slate-500" title="Draft mutation workflow is not enabled">Create draft</button></div></div>
    <form id="search-form" class="flex flex-col gap-3 border-b border-slate-100 bg-slate-50/70 p-4 sm:flex-row"><input id="search" value="${escape(search)}" placeholder="Search this object catalog..." class="min-w-0 flex-1 rounded-lg border border-slate-200 bg-white px-3 py-2.5 text-sm outline-none focus:ring-2 focus:ring-cyan-500"/><button class="rounded-lg bg-slate-900 px-4 py-2.5 text-sm font-semibold text-white">Search</button><select id="page-size" class="rounded-lg border border-slate-200 bg-white px-3 py-2.5 text-sm"><option ${pageSize === 10 ? "selected" : ""}>10</option><option ${pageSize === 25 ? "selected" : ""}>25</option><option ${pageSize === 50 ? "selected" : ""}>50</option><option ${pageSize === 100 ? "selected" : ""}>100</option></select></form>
    <div class="overflow-x-auto"><table class="w-full min-w-[900px] text-left text-sm"><thead class="bg-slate-50 text-[10px] font-bold uppercase tracking-[.12em] text-slate-500"><tr>${config.columns.map((column) => `<th class="whitespace-nowrap px-5 py-3">${column.label}</th>`).join("")}</tr></thead><tbody class="divide-y divide-slate-100">${data.data.length ? data.data.map((row) => tableRow(config, row)).join("") : `<tr><td colspan="${config.columns.length}" class="px-5 py-14 text-center text-slate-500">No objects match this query.</td></tr>`}</tbody></table></div>${pagination(data)}</section>`;
}

function wikiView() {
  if (!wikiData) return empty("Weapon Wiki data is unavailable.");
  const entries = wikiData.entries.filter((entry) => {
    const needle = search.toLowerCase();
    const matchesSearch = !needle || [entry.id, entry.nameEn, entry.nameVi, entry.family, entry.exclusiveGroup].some((value) => value.toLowerCase().includes(needle));
    const matchesSlot = wikiSlot === "all" || entry.slot === wikiSlot;
    const matchesTier = wikiTier === "all" || entry.tiers.some((tier) => tier.tier === Number(wikiTier));
    return matchesSearch && matchesSlot && matchesTier;
  });
  const prefix = entries.filter((entry) => entry.slot === "prefix");
  const suffix = entries.filter((entry) => entry.slot === "suffix");
  return `<section class="overflow-hidden rounded-2xl border border-slate-200 bg-white shadow-sm"><div class="border-b border-slate-200 bg-gradient-to-br from-[#0b1727] via-[#10253a] to-[#0d5262] p-6 text-white sm:p-8"><div class="flex flex-col gap-5 lg:flex-row lg:items-end lg:justify-between"><div><p class="text-[10px] font-bold uppercase tracking-[.2em] text-cyan-300">Reference library</p><h2 class="mt-2 text-3xl font-bold tracking-tight">Weapon modifier wiki</h2><p class="mt-2 max-w-2xl text-sm leading-6 text-slate-300">Click any modifier row to inspect every tier and roll range. Base weapon only sets the level ceiling.</p></div><div class="rounded-xl border border-white/15 bg-white/10 px-4 py-3 text-xs text-slate-200"><p class="font-semibold text-white">${number(entries.length)} modifiers shown</p><p class="mt-1 text-slate-400">Release ${escape(wikiData.releaseId)}</p></div></div></div><form id="wiki-filter" class="flex flex-col gap-3 border-b border-slate-100 bg-slate-50/80 p-4 sm:flex-row"><input id="wiki-search" value="${escape(search)}" placeholder="Search modifier, family or source..." class="min-w-0 flex-1 rounded-lg border border-slate-200 bg-white px-3 py-2.5 text-sm outline-none focus:ring-2 focus:ring-cyan-500"/><select id="wiki-slot" class="rounded-lg border border-slate-200 bg-white px-3 py-2.5 text-sm"><option value="all" ${wikiSlot === "all" ? "selected" : ""}>All slots</option><option value="prefix" ${wikiSlot === "prefix" ? "selected" : ""}>Prefix only</option><option value="suffix" ${wikiSlot === "suffix" ? "selected" : ""}>Suffix only</option></select><select id="wiki-tier" class="rounded-lg border border-slate-200 bg-white px-3 py-2.5 text-sm"><option value="all" ${wikiTier === "all" ? "selected" : ""}>All tiers</option>${Array.from({length: 8}, (_, i) => `<option value="${i + 1}" ${wikiTier === String(i + 1) ? "selected" : ""}>T${i + 1} · req. ${i * 100}</option>`).join("")}</select><button class="rounded-lg bg-slate-900 px-4 py-2.5 text-sm font-semibold text-white">Find</button></form><div class="space-y-6 p-4 sm:p-6">${wikiTable("Prefix", prefix, "amber")}${wikiTable("Suffix", suffix, "cyan")}</div></section>`;
}

function wikiTable(label: string, entries: WikiEntry[], tone: "amber" | "cyan") {
  const heading = tone === "amber" ? "text-amber-700" : "text-cyan-700";
  return `<section class="overflow-hidden rounded-2xl border border-slate-200 bg-white"><div class="flex items-center justify-between border-b border-slate-100 bg-slate-50/70 px-5 py-4"><div><h3 class="text-lg font-bold ${heading}">${label}</h3><p class="mt-1 text-xs text-slate-500">${entries.length} modifiers · click a row for tier details</p></div><span class="rounded-full bg-slate-900 px-3 py-1 text-xs font-bold text-white">${label === "Prefix" ? "P" : "S"}</span></div><div class="overflow-x-auto"><table class="w-full min-w-[900px] text-left text-sm"><thead class="bg-white text-[10px] font-bold uppercase tracking-[.12em] text-slate-400"><tr><th class="px-5 py-3">Modifier</th><th class="px-5 py-3">Vietnamese</th><th class="px-5 py-3">Family</th><th class="px-5 py-3">Exclusive group</th><th class="px-5 py-3">Weight</th><th class="px-5 py-3">Source</th></tr></thead><tbody class="divide-y divide-slate-100">${entries.length ? entries.map(wikiRow).join("") : `<tr><td colspan="6" class="px-5 py-12 text-center text-slate-500">No modifiers match these filters.</td></tr>`}</tbody></table></div></section>`;
}

function wikiRow(entry: WikiEntry) {
  return `<tr data-wiki-id="${escape(entry.id)}" tabindex="0" role="button" class="cursor-pointer transition hover:bg-cyan-50/50 focus:bg-cyan-50 focus:outline-none focus:ring-2 focus:ring-inset focus:ring-cyan-400" title="Click to inspect modifier tiers"><td class="px-4 py-3 font-semibold text-slate-900">${escape(entry.nameEn)}<span class="ml-2 font-mono text-[10px] font-normal text-slate-400">${escape(entry.id)}</span></td><td class="px-4 py-3 text-slate-500">${escape(entry.nameVi)}</td><td class="px-4 py-3 text-slate-600">${escape(entry.family)}</td><td class="px-4 py-3 text-slate-600">${escape(entry.exclusiveGroup)}</td><td class="px-4 py-3 font-mono font-bold text-slate-800">${number(entry.weight)}</td><td class="px-4 py-3 text-xs text-slate-500">T1–T${entry.tiers.length}<span class="ml-2 text-cyan-600">↗</span></td></tr>`;
}

function emptyWikiCard() { return '<div class="rounded-xl border border-dashed border-slate-300 p-12 text-center text-sm text-slate-500 xl:col-span-2">No modifiers match these filters.</div>'; }

function tableRow(config: PageConfig, row: JsonRow) { return `<tr data-inspect='${escape(JSON.stringify(row))}' tabindex="0" role="button" class="cursor-pointer transition hover:bg-cyan-50/50 focus:bg-cyan-50 focus:outline-none focus:ring-2 focus:ring-inset focus:ring-cyan-400">${config.columns.map((column, index) => { const value = column.format ? column.format(row[column.key], row) : number(row[column.key]); const imageUrl = column.key === "inventoryIcon" && row[column.key] ? `/evil-admin/legacy-assets/${String(value).split("/").pop()}` : ""; const image = imageUrl ? `<img src="${escape(imageUrl)}" alt="" class="h-10 w-10 rounded-lg border border-slate-200 bg-slate-100 object-contain p-1" loading="lazy"/>` : escape(value); return `<td class="max-w-[340px] ${column.key === "inventoryIcon" ? "w-16" : ""} ${index === 0 ? "font-semibold text-slate-900" : "text-slate-600"} truncate px-4 py-3" title="${escape(value)}">${image}</td>`; }).join("")}</tr>`; }

function pagination(data: PageResponse) {
  const from = data.total === 0 ? 0 : (data.page - 1) * data.pageSize + 1;
  const to = Math.min(data.page * data.pageSize, data.total);
  return `<div class="flex flex-col gap-3 border-t border-slate-100 px-5 py-4 text-xs text-slate-500 sm:flex-row sm:items-center sm:justify-between"><span>Showing ${number(from)}–${number(to)} of ${number(data.total)} · Page ${data.page} of ${Math.max(data.totalPages, 1)}</span><div class="flex gap-2"><button id="previous" ${data.page <= 1 ? "disabled" : ""} class="rounded-lg border border-slate-200 px-3 py-2 font-semibold disabled:cursor-not-allowed disabled:opacity-40">Previous</button><button id="next" ${data.page >= data.totalPages ? "disabled" : ""} class="rounded-lg border border-slate-200 px-3 py-2 font-semibold disabled:cursor-not-allowed disabled:opacity-40">Next</button></div></div>`;
}

function skeleton() { return `<div class="space-y-3 rounded-2xl border border-slate-200 bg-white p-6">${Array.from({ length: 7 }, () => '<div class="h-12 animate-pulse rounded-lg bg-slate-100"></div>').join("")}</div>`; }
function empty(message: string) { return `<div class="rounded-2xl border border-dashed border-slate-300 bg-white p-12 text-center text-sm text-slate-500">${message}</div>`; }
function showError() { app.insertAdjacentHTML("afterbegin", '<div class="fixed right-5 top-5 z-30 rounded-xl bg-red-600 px-4 py-3 text-sm font-semibold text-white shadow-xl">Admin API request failed.</div>'); }

function bind() {
  document.querySelector("#logout")?.addEventListener("click", () => { password = ""; showLogin(); });
  document.querySelector("#refresh")?.addEventListener("click", () => void load());
  document.querySelector("#search-form")?.addEventListener("submit", (event) => { event.preventDefault(); search = (document.querySelector<HTMLInputElement>("#search")?.value ?? "").trim(); currentPage = 1; void load(); });
  document.querySelector("#page-size")?.addEventListener("change", (event) => { pageSize = Number((event.target as HTMLSelectElement).value); currentPage = 1; void load(); });
  document.querySelector("#previous")?.addEventListener("click", () => { currentPage--; void load(); });
  document.querySelector("#next")?.addEventListener("click", () => { currentPage++; void load(); });
  document.querySelector("#wiki-filter")?.addEventListener("submit", (event) => { event.preventDefault(); search = (document.querySelector<HTMLInputElement>("#wiki-search")?.value ?? "").trim(); wikiSlot = document.querySelector<HTMLSelectElement>("#wiki-slot")?.value ?? "all"; wikiTier = document.querySelector<HTMLSelectElement>("#wiki-tier")?.value ?? "all"; render(); });
  document.querySelectorAll<HTMLElement>("[data-wiki-id]").forEach((row) => { const open = () => { const entry = wikiData?.entries.find((candidate) => candidate.id === row.dataset.wikiId); if (entry) showWikiInspector(entry); }; row.addEventListener("click", open); row.addEventListener("keydown", (event) => { if (event.key === "Enter" || event.key === " ") { event.preventDefault(); open(); } }); });
  document.querySelectorAll<HTMLElement>("[data-inspect]").forEach((row) => { const open = () => showInspector(JSON.parse(row.dataset.inspect!)); row.addEventListener("click", open); row.addEventListener("keydown", (event) => { if (event.key === "Enter" || event.key === " ") { event.preventDefault(); open(); } }); });
}

function showInspector(row: JsonRow) {
  const modal = document.createElement("div"); modal.className = "fixed inset-0 z-30 flex justify-end bg-slate-950/40";
  modal.innerHTML = `<div class="h-full w-full max-w-xl overflow-y-auto bg-white p-7 shadow-2xl"><div class="flex items-start justify-between"><div><p class="text-[10px] font-bold uppercase tracking-[.16em] text-cyan-600">Authoritative object</p><h2 class="mt-1 text-xl font-bold text-slate-950">Object details</h2></div><button class="close text-2xl text-slate-400">&times;</button></div><dl class="mt-7 divide-y divide-slate-100">${Object.entries(row).map(([key, value]) => `<div class="grid grid-cols-[140px_1fr] gap-4 py-4"><dt class="text-xs font-bold uppercase tracking-wide text-slate-400">${escape(key)}</dt><dd class="break-words font-mono text-sm text-slate-700">${escape(typeof value === "object" ? JSON.stringify(value) : value)}</dd></div>`).join("")}</dl></div>`;
  document.body.append(modal); modal.addEventListener("click", (event) => { if (event.target === modal || (event.target as HTMLElement).closest(".close")) modal.remove(); });
}

function showWikiInspector(entry: WikiEntry) {
  const visibleTiers = wikiTier === "all" ? entry.tiers : entry.tiers.filter((tier) => tier.tier === Number(wikiTier));
  const modal = document.createElement("div"); modal.className = "fixed inset-0 z-30 flex items-center justify-center bg-slate-950/55 p-4";
  modal.innerHTML = `<div class="max-h-[90vh] w-full max-w-4xl overflow-y-auto rounded-2xl bg-white shadow-2xl"><div class="sticky top-0 flex items-start justify-between gap-4 border-b border-slate-200 bg-white p-6"><div><p class="text-[10px] font-bold uppercase tracking-[.16em] text-cyan-600">Modifier detail</p><h2 class="mt-1 text-2xl font-bold text-slate-950">${escape(entry.nameEn)}</h2><p class="mt-1 text-sm text-slate-500">${escape(entry.nameVi)}</p></div><button class="close rounded-lg px-3 py-1 text-2xl text-slate-400 hover:bg-slate-100">&times;</button></div><div class="grid gap-3 border-b border-slate-100 bg-slate-50/70 p-6 sm:grid-cols-4"><div><p class="wiki-label">Slot</p><p class="mt-1 font-semibold text-slate-800">${escape(entry.slot)}</p></div><div><p class="wiki-label">Weight</p><p class="mt-1 font-mono font-semibold text-slate-800">${number(entry.weight)}</p></div><div><p class="wiki-label">Family</p><p class="mt-1 font-semibold text-slate-800">${escape(entry.family)}</p></div><div><p class="wiki-label">Exclusive group</p><p class="mt-1 font-semibold text-slate-800">${escape(entry.exclusiveGroup)}</p></div></div><div class="overflow-x-auto p-6"><table class="w-full min-w-[640px] text-left text-sm"><thead class="text-[10px] font-bold uppercase tracking-[.12em] text-slate-400"><tr><th class="pb-3">Tier</th><th class="pb-3">Required item level</th><th class="pb-3">Range</th><th class="pb-3">Basis</th></tr></thead><tbody class="divide-y divide-slate-100">${visibleTiers.map((tier) => `<tr><td class="py-3 font-bold text-slate-800">T${tier.tier}</td><td class="py-3 text-slate-600">${tier.requiredItemLevel}+</td><td class="py-3"><span class="rounded-md bg-slate-900 px-2 py-1 font-mono font-bold text-white">${tier.minimumValue}–${tier.maximumValue}</span></td><td class="py-3 text-slate-500">${escape(tier.valueBasis)}</td></tr>`).join("")}</tbody></table></div></div>`;
  document.body.append(modal); modal.addEventListener("click", (event) => { if (event.target === modal || (event.target as HTMLElement).closest(".close")) modal.remove(); });
}

function showLogin() {
  if (document.querySelector("#admin-login")) return;
  const modal = document.createElement("div"); modal.id = "admin-login"; modal.className = "fixed inset-0 z-40 grid place-items-center bg-[#07111f]/90 p-4";
  modal.innerHTML = `<form class="w-full max-w-sm rounded-2xl bg-white p-7 shadow-2xl"><div class="mb-6 grid h-11 w-11 place-items-center rounded-xl bg-cyan-400 font-black text-slate-950">EH</div><h2 class="text-xl font-bold text-slate-950">Operations sign in</h2><p class="mt-1 text-sm text-slate-500">Temporary Basic Authentication</p><label class="mt-6 block text-xs font-semibold text-slate-600">Username<input name="username" value="${escape(username)}" autocomplete="username" class="mt-2 w-full rounded-lg border border-slate-200 px-3 py-2.5 text-sm" required></label><label class="mt-4 block text-xs font-semibold text-slate-600">Password<input name="password" type="password" autocomplete="current-password" class="mt-2 w-full rounded-lg border border-slate-200 px-3 py-2.5 text-sm" required></label><p class="error mt-3 hidden text-xs font-semibold text-red-600">Invalid credentials.</p><button class="mt-6 w-full rounded-lg bg-slate-900 px-4 py-2.5 text-sm font-semibold text-white">Continue</button></form>`;
  document.body.append(modal);
  modal.querySelector("form")!.addEventListener("submit", async (event) => { event.preventDefault(); const form = new FormData(event.currentTarget as HTMLFormElement); username = String(form.get("username")); password = String(form.get("password")); localStorage.setItem("admin_user", username); try { await api<Overview>("overview"); modal.remove(); await load(); } catch { password = ""; modal.querySelector(".error")?.classList.remove("hidden"); } });
}

render();
showLogin();
