<script lang="ts">
	import { onMount, tick } from "svelte";
	import {
		getApps,
		getExtensionsForApp,
		getCandidateTargets,
		getExtensionTargetCounts,
		getEligibleExtensions,
		reassignExtensions,
		getAppsForExtension,
		getAppsForExtensions,
		getSummary,
	} from "./lib/api";
	import type { App, Extension } from "./lib/types";
	import { resolveAction } from "./lib/keymap";
	import type { Action } from "./lib/keymap";

	let apps: App[] = $state([]);
	let allExtensions: Extension[] = $state([]);
	let extensions: Extension[] = $state([]);
	let targetExtensions: Extension[] = $state([]);
	let lockedExtensions: Extension[] | null = $state(null);
	let candidateTargets: App[] = $state([]);
	let filteredTargets: App[] = $state([]);
	let eligibleExts: Set<string> = $state(new Set());
	let extTargetCounts: Map<string, number> = $state(new Map());
	let selectedExts: Set<string> = $state(new Set());
	let summary: [number, number] = $state([0, 0]);

	let extFilter = $state("");
	let selectedSourceId: number | null = $state(null);
	let selectedTargetId: number | null = $state(null);

	let appSort = $state<"alpha" | "ext_count">("ext_count");
	let loading = $state(true);
	let browseAll = $state(false);
	let targetFilter = $state("");
	let allApps: App[] = $state([]);
	let targetFilterInputEl: HTMLInputElement | undefined = $state(undefined);

	type Panel = "apps" | "extensions" | "targets";
	let focusedPanel: Panel = $state("apps");
	let extCursor = $state(0);

	let extFilterInputEl: HTMLInputElement | undefined = $state(undefined);
	let panelBodyEls: Record<Panel, HTMLElement | undefined> = $state({
		apps: undefined,
		extensions: undefined,
		targets: undefined,
	});

	let filteredExtensions = $derived.by(() => {
		if (lockedExtensions !== null) return lockedExtensions;

		let base: Extension[];
		if (extFilter) {
			base = allExtensions;
		} else if (eligibleExts.size > 0) {
			base = extensions.filter((e) => eligibleExts.has(e.ext));
		} else if (selectedTargetId !== null) {
			base = [];
		} else {
			base = extensions;
		}

		if (extFilter) {
			const q = extFilter.toLowerCase();
			return base.filter(
				(e) =>
					e.ext.includes(q) ||
					e.description.toLowerCase().includes(q),
			);
		}
		return base;
	});

	let visibleApps = $derived.by(() => {
		if (!extFilter) return apps;
		const ownerIds = new Set(
			filteredExtensions
				.map((e) => e.default_app_id)
				.filter((id) => id !== null),
		);
		return apps.filter((a) => ownerIds.has(a.id));
	});

	let sortedApps = $derived(
		appSort === "alpha"
			? visibleApps
			: [...visibleApps].sort((a, b) => b.ext_count - a.ext_count),
	);

	let sortedExtensions = $derived.by(() => {
		if (selectedTargetId !== null || extFilter || extTargetCounts.size === 0) return filteredExtensions;
		return [...filteredExtensions].sort((a, b) => {
			const ca = extTargetCounts.get(a.ext) ?? 0;
			const cb = extTargetCounts.get(b.ext) ?? 0;
			return cb - ca;
		});
	});

	let appCursor = $derived(
		selectedSourceId !== null
			? Math.max(0, sortedApps.findIndex((a) => a.id === selectedSourceId))
			: 0,
	);

	let displayedTargets = $derived.by(() => {
		if (!browseAll) return filteredTargets;
		const src = selectedSourceId;
		const list = allApps.filter((a) => a.id !== src);
		if (!targetFilter) return list;
		const q = targetFilter.toLowerCase();
		return list.filter((a) => a.name.toLowerCase().includes(q));
	});

	let targetCursor = $state(0);

	let canReassign = $derived(selectedExts.size > 0 && selectedTargetId !== null && selectedSourceId !== null);
	let reassignSourceApp = $derived(canReassign ? apps.find((a) => a.id === selectedSourceId) : null);
	let reassignTargetApp = $derived(canReassign ? displayedTargets.find((a) => a.id === selectedTargetId) : null);

	let selectedOwnerAppIds = $derived.by(() => {
		if (selectedExts.size === 0) return new Set<number>();
		const ids = new Set<number>();
		for (const e of sortedExtensions) {
			if (selectedExts.has(e.ext) && e.default_app_id !== null) {
				ids.add(e.default_app_id);
			}
		}
		return ids;
	});

	async function refresh() {
		const [a, all, s] = await Promise.all([
			getApps(),
			getExtensionsForApp(),
			getSummary(),
		]);
		apps = a;
		allExtensions = all;
		summary = s;
	}

	async function init() {
		loading = true;
		await refresh();
		if (sortedApps.length > 0) {
			await selectSource(sortedApps[0].id);
		}
		loading = false;
	}

	onMount(() => {
		init();
	});

	async function toggleAppSort() {
		const body = panelBodyEls.apps;
		let relativeOffset: number | null = null;
		const active = body?.querySelector(".app-item.active") as
			| HTMLElement
			| null;

		if (body && active) {
			const bodyRect = body.getBoundingClientRect();
			const childRect = active.getBoundingClientRect();
			relativeOffset = childRect.top - bodyRect.top;
		}

		appSort = appSort === "alpha" ? "ext_count" : "alpha";

		if (body && relativeOffset !== null) {
			await tick();
			const active = body.querySelector(".app-item.active") as
				| HTMLElement
				| null;
			if (active) {
				const bodyRect = body.getBoundingClientRect();
				const childRect = active.getBoundingClientRect();
				body.scrollTop +=
					childRect.top - bodyRect.top - relativeOffset;
			}
		}
	}

	async function selectSource(appId: number | null) {
		selectedSourceId = appId;
		selectedTargetId = null;
		selectedExts = new Set();
		eligibleExts = new Set();
		targetExtensions = [];
		lockedExtensions = null;
		extCursor = 0;
		targetCursor = 0;
		browseAll = false;
		targetFilter = "";

		if (appId !== null) {
			const [exts, targets, counts] = await Promise.all([
				getExtensionsForApp(appId),
				getCandidateTargets(appId),
				getExtensionTargetCounts(appId),
			]);
			extensions = exts;
			candidateTargets = targets;
			filteredTargets = targets;
			extTargetCounts = new Map(counts);
		} else {
			extensions = await getExtensionsForApp();
			candidateTargets = [];
			filteredTargets = [];
			extTargetCounts = new Map();
		}
	}

	async function switchSourceKeepState(appId: number) {
		if (appId === selectedSourceId) return;
		selectedSourceId = appId;
		selectedTargetId = null;
		eligibleExts = new Set();
		targetExtensions = [];

		const [exts, targets, counts] = await Promise.all([
			getExtensionsForApp(appId),
			getCandidateTargets(appId),
			getExtensionTargetCounts(appId),
		]);
		extensions = exts;
		candidateTargets = targets;
		filteredTargets = targets;
		extTargetCounts = new Map(counts);
	}

	async function selectTarget(appId: number | null) {
		selectedTargetId = appId;

		if (appId !== null && selectedSourceId !== null) {
			const [eligible, tExts] = await Promise.all([
				getEligibleExtensions(selectedSourceId, appId),
				getExtensionsForApp(appId),
			]);
			eligibleExts = new Set(eligible);
			if (!browseAll) targetExtensions = tExts;
		} else {
			eligibleExts = new Set();
			if (!browseAll) targetExtensions = [];
		}
	}

	async function ensureSource(extData: Extension) {
		if (
			extData.default_app_id !== null &&
			extData.default_app_id !== selectedSourceId
		) {
			await switchSourceKeepState(extData.default_app_id);
		}
	}

	async function toggleExt(ext: string, event: MouseEvent) {
		const extData = sortedExtensions.find((e) => e.ext === ext);
		const removing = selectedExts.has(ext);

		if (!removing && extData) await ensureSource(extData);

		const next = new Set(selectedExts);
		if (event.metaKey || event.ctrlKey) {
			if (next.has(ext)) next.delete(ext);
			else next.add(ext);
		} else if (event.shiftKey) {
			const allExts = sortedExtensions.map((e) => e.ext);
			const lastSelected = [...selectedExts].pop();
			if (lastSelected) {
				const from = allExts.indexOf(lastSelected);
				const to = allExts.indexOf(ext);
				const [start, end] = from < to ? [from, to] : [to, from];
				for (let i = start; i <= end; i++) next.add(allExts[i]);
			} else {
				next.add(ext);
			}
		} else {
			if (next.size === 1 && next.has(ext)) {
				next.clear();
			} else {
				next.clear();
				next.add(ext);
			}
		}
		selectedExts = next;
		await refreshTargets(next);
	}

	async function refreshTargets(exts: Set<string>) {
		const prevTargetId = displayedTargets[targetCursor]?.id ?? null;

		if (exts.size > 0 && selectedSourceId !== null) {
			filteredTargets = await getAppsForExtensions(
				[...exts],
				selectedSourceId,
			);
		} else {
			filteredTargets = candidateTargets;
			if (selectedTargetId !== null && selectedSourceId !== null) {
				const eligible = await getEligibleExtensions(selectedSourceId, selectedTargetId);
				eligibleExts = new Set(eligible);
			} else {
				eligibleExts = new Set();
			}
		}
		// If current target is no longer valid, deselect it
		if (
			selectedTargetId !== null &&
			!filteredTargets.some((a) => a.id === selectedTargetId)
		) {
			selectedTargetId = null;
			eligibleExts = new Set();
		}
		// Preserve cursor position by app id, or clamp
		if (prevTargetId !== null) {
			const idx = displayedTargets.findIndex((a) => a.id === prevTargetId);
			targetCursor = idx >= 0 ? idx : Math.min(targetCursor, Math.max(0, displayedTargets.length - 1));
		} else {
			targetCursor = Math.min(targetCursor, Math.max(0, displayedTargets.length - 1));
		}
	}

	async function toggleBrowseAll() {
		browseAll = !browseAll;
		targetFilter = "";

		if (browseAll) {
			lockedExtensions = [...sortedExtensions];
			if (allApps.length === 0) {
				allApps = await getApps();
			}
		} else {
			lockedExtensions = null;
		}

		selectedTargetId = null;
		eligibleExts = new Set();
		await tick();
		if (browseAll) targetFilterInputEl?.focus();
	}

	function selectAllEligible() {
		selectedExts = new Set(sortedExtensions.map((e) => e.ext));
	}

	async function doReassign() {
		if (
			selectedTargetId === null ||
			selectedSourceId === null ||
			selectedExts.size === 0
		)
			return;
		const exts = [...selectedExts];
		await reassignExtensions(exts, selectedTargetId);
		selectedExts = new Set();
		selectedTargetId = null;
		eligibleExts = new Set();
		await refresh();
		await selectSource(selectedSourceId);
	}

	function panelListLength(panel: Panel): number {
		if (panel === "apps") return sortedApps.length;
		if (panel === "extensions") return sortedExtensions.length;
		if (panel === "targets") return displayedTargets.length;
		return 0;
	}

	function cursorFor(panel: Panel): number {
		if (panel === "apps") return appCursor;
		if (panel === "extensions") return extCursor;
		return targetCursor;
	}

	function clampExtCursor() {
		const len = sortedExtensions.length;
		if (len === 0) extCursor = 0;
		else if (extCursor >= len) extCursor = len - 1;
	}

	function scrollCursorIntoView(panel: Panel) {
		const body = panelBodyEls[panel];
		if (!body) return;
		const idx = cursorFor(panel);
		const child = body.children[idx] as HTMLElement | undefined;
		child?.scrollIntoView({ block: "nearest" });
	}

	function scrollHalfPage(panel: Panel, direction: number) {
		const body = panelBodyEls[panel];
		if (!body) return;
		body.scrollBy({
			top: direction * (body.clientHeight / 2),
			behavior: "smooth",
		});
	}

	async function selectPanelItem(panel: Panel, index: number) {
		if (panel === "apps") {
			const appId = sortedApps[index]?.id;
			if (appId !== undefined) {
				if (extFilter) await switchSourceKeepState(appId);
				else await selectSource(appId);
			}
		} else if (panel === "extensions") {
			const extData = sortedExtensions[index];
			if (extData) {
				extCursor = index;
				await ensureSource(extData);
				const next = new Set<string>();
				next.add(extData.ext);
				selectedExts = next;
				await refreshTargets(next);
			}
		} else if (panel === "targets") {
			targetCursor = index;
			const appId = displayedTargets[index]?.id;
			if (appId !== undefined) await selectTarget(appId);
		}
	}

	async function handleAction(action: Action) {
		const panels: Panel[] = ["apps", "extensions", "targets"];
		const panelIdx = panels.indexOf(focusedPanel);

		switch (action) {
			case "focus_left":
				if (panelIdx > 0) {
					focusedPanel = panels[panelIdx - 1];
					if (focusedPanel === "extensions") {
						clampExtCursor();
						await selectPanelItem("extensions", extCursor);
					}
				}
				break;
			case "focus_right":
				if (panelIdx < panels.length - 1) {
					focusedPanel = panels[panelIdx + 1];
					if (focusedPanel === "extensions") {
						clampExtCursor();
						await selectPanelItem("extensions", extCursor);
					} else if (focusedPanel === "targets") {
						const idx = cursorFor("targets");
						await selectPanelItem("targets", idx);
					}
				}
				break;

			case "move_down": {
				const len = panelListLength(focusedPanel);
				const cur = cursorFor(focusedPanel);
				if (cur < len - 1) {
					await selectPanelItem(focusedPanel, cur + 1);
					scrollCursorIntoView(focusedPanel);
				}
				break;
			}
			case "move_up": {
				const cur = cursorFor(focusedPanel);
				if (cur > 0) {
					await selectPanelItem(focusedPanel, cur - 1);
					scrollCursorIntoView(focusedPanel);
				}
				break;
			}
			case "move_top": {
				await selectPanelItem(focusedPanel, 0);
				scrollCursorIntoView(focusedPanel);
				break;
			}
			case "move_bottom": {
				const len = panelListLength(focusedPanel);
				if (len > 0) {
					await selectPanelItem(focusedPanel, len - 1);
				}
				scrollCursorIntoView(focusedPanel);
				break;
			}

			case "select":
				await selectPanelItem(
					focusedPanel,
					cursorFor(focusedPanel),
				);
				break;

			case "toggle_select":
				if (focusedPanel === "extensions") {
					const extData = sortedExtensions[extCursor];
					if (extData) {
						if (!selectedExts.has(extData.ext)) await ensureSource(extData);
						const next = new Set(selectedExts);
						if (next.has(extData.ext)) next.delete(extData.ext);
						else next.add(extData.ext);
						selectedExts = next;
						await refreshTargets(next);
					}
				}
				break;

			case "extend_down":
				if (focusedPanel === "extensions") {
					const extData = sortedExtensions[extCursor];
					if (extData) {
						await ensureSource(extData);
						const next = new Set(selectedExts);
						next.add(extData.ext);
						selectedExts = next;
					}
					if (extCursor < sortedExtensions.length - 1) {
						extCursor++;
						const nextData = sortedExtensions[extCursor];
						if (nextData) {
							const next2 = new Set(selectedExts);
							next2.add(nextData.ext);
							selectedExts = next2;
						}
						scrollCursorIntoView("extensions");
					}
					await refreshTargets(selectedExts);
				}
				break;

			case "extend_up":
				if (focusedPanel === "extensions") {
					const extData = sortedExtensions[extCursor];
					if (extData) {
						await ensureSource(extData);
						const next = new Set(selectedExts);
						next.add(extData.ext);
						selectedExts = next;
					}
					if (extCursor > 0) {
						extCursor--;
						const nextData = sortedExtensions[extCursor];
						if (nextData) {
							const next2 = new Set(selectedExts);
							next2.add(nextData.ext);
							selectedExts = next2;
						}
						scrollCursorIntoView("extensions");
					}
					await refreshTargets(selectedExts);
				}
				break;

			case "select_all":
				if (focusedPanel === "extensions") {
					selectAllEligible();
				}
				break;

			case "reassign":
				await doReassign();
				break;

			case "search":
				extFilterInputEl?.focus();
				break;

			case "escape":
				if (document.activeElement === extFilterInputEl) {
					extFilter = "";
					extFilterInputEl?.blur();
					extCursor = 0;
				} else if (selectedTargetId !== null) {
					await selectTarget(null);
				} else {
					selectedExts = new Set();
				}
				break;

			case "scroll_half_down":
				scrollHalfPage(focusedPanel, 1);
				break;
			case "scroll_half_up":
				scrollHalfPage(focusedPanel, -1);
				break;
		}
	}

	function onGlobalKeydown(e: KeyboardEvent) {
		const tag = (e.target as HTMLElement)?.tagName;
		if (tag === "INPUT" || tag === "SELECT" || tag === "TEXTAREA") {
			if (e.key === "Escape") {
				handleAction("escape");
				e.preventDefault();
			}
			return;
		}

		const action = resolveAction(e);
		if (action) {
			e.preventDefault();
			handleAction(action);
		}
	}
</script>

<svelte:window onkeydown={onGlobalKeydown} />

<main>
	{#if loading}
		<div class="loading">
			<div class="loading-dots">
				<span></span><span></span><span></span>
			</div>
			<span class="loading-text">Scanning applications</span>
		</div>
	{:else}
		<div class="panels">
			<!-- Left: Source Apps -->
			<div
				class="panel apps-panel"
				class:panel-focused={focusedPanel === "apps"}
			>
				<div class="panel-header">
					<h2>Apps</h2>
					<button
						class="sort-toggle"
						onclick={toggleAppSort}
						title={appSort === "alpha"
							? "Sorted A-Z; click to sort by default count"
							: "Sorted by default count; click to sort A-Z"}
					>
						{appSort === "alpha" ? "A-Z" : "#Ext"}
					</button>
				</div>
				<div class="panel-body" bind:this={panelBodyEls.apps}>
					{#each sortedApps as app, i (app.id)}
						<button
							class="app-item"
							class:cursor={appCursor === i}
							class:active={selectedSourceId === app.id}
							class:owner-highlight={selectedOwnerAppIds.has(app.id)}
							onclick={() => {
								focusedPanel = "apps";
								if (extFilter) {
									switchSourceKeepState(app.id);
								} else {
									selectSource(app.id);
								}
							}}
						>
							<span class="app-name">{app.name}</span>
							<span class="badge">{app.ext_count}</span>
						</button>
					{/each}
				</div>
			</div>

			<!-- Middle: Extensions -->
			<div
				class="panel extensions-panel"
				class:panel-focused={focusedPanel === "extensions"}
			>
				<div class="panel-header">
					<h2>Extensions</h2>
					<div class="search-box">
						<input
							type="text"
							placeholder="Search all extensions..."
							bind:value={extFilter}
							bind:this={extFilterInputEl}
							oninput={() => { extCursor = 0; selectedExts = new Set(); selectedSourceId = null; selectedTargetId = null; eligibleExts = new Set(); }}
						/>
						{#if extFilter}
							<button
								class="search-clear"
								onclick={() => { extFilter = ""; extCursor = 0; }}
								title="Clear search"
							>&times;</button>
						{/if}
					</div>
					<button onclick={selectAllEligible}>
						Select All ({sortedExtensions.length})
					</button>
				</div>
				<div class="panel-body" bind:this={panelBodyEls.extensions}>
					{#each sortedExtensions as ext, i (ext.ext)}
						<div
							class="ext-item"
							class:selected={selectedExts.has(ext.ext)}
							class:dimmed={selectedTargetId === null && !extFilter && extTargetCounts.size > 0 && (extTargetCounts.get(ext.ext) ?? 0) === 0}
							class:cursor={extCursor === i}
							role="option"
							aria-selected={selectedExts.has(ext.ext)}
							tabindex="0"
							onclick={(e) => {
								focusedPanel = "extensions";
								extCursor = i;
								toggleExt(ext.ext, e);
							}}
							onkeydown={(e) =>
								e.key === "Enter" &&
								toggleExt(
									ext.ext,
									e as unknown as MouseEvent,
								)}
						>
							<span class="ext-name">.{ext.ext}</span>
							{#if ext.description}
								<span class="ext-desc"
									>{ext.description}</span
								>
							{/if}
							{#if (extFilter || selectedSourceId === null) && ext.default_app_name}
								<span class="ext-default"
									>{ext.default_app_name}</span
								>
							{/if}
						</div>
					{/each}
					{#if sortedExtensions.length === 0}
						<div class="empty">
							<span class="empty-icon">&#x2205;</span>
							<span class="empty-label">{extFilter ? 'No matching extensions' : 'No extensions for this app'}</span>
						</div>
					{/if}
				</div>
				</div>

			<!-- Right: Target Apps -->
			{#if selectedSourceId !== null}
				<div
					class="panel targets-panel"
					class:panel-focused={focusedPanel === "targets"}
				>
					<div class="panel-header">
						<h2>Reassign To</h2>
						{#if browseAll}
							<div class="search-box">
								<input
									type="text"
									placeholder="Filter apps..."
									bind:value={targetFilter}
									bind:this={targetFilterInputEl}
								/>
								{#if targetFilter}
									<button class="search-clear" onclick={() => { targetFilter = ""; targetFilterInputEl?.focus(); }}>&times;</button>
								{/if}
							</div>
						{/if}
						<button
							class="any-toggle"
							class:any-active={browseAll}
							onclick={toggleBrowseAll}
						>any</button>
					</div>
					<div
						class="panel-body"
						bind:this={panelBodyEls.targets}
					>
						{#each displayedTargets as app, i (app.id)}
							<button
								class="app-item"
								class:cursor={targetCursor === i}
								class:active={selectedTargetId === app.id}
								onclick={() => {
									focusedPanel = "targets";
									targetCursor = i;
									selectTarget(selectedTargetId === app.id ? null : app.id);
								}}
							>
								<span class="app-name">{app.name}</span>
								<span class="badge"
									>{app.ext_count}</span
								>
							</button>
						{/each}
						{#if displayedTargets.length === 0}
							<div class="empty">
								<span class="empty-icon">&#x21A6;</span>
								<span class="empty-label">{browseAll ? 'No matching apps' : selectedExts.size === 1 ? `No other apps for .${[...selectedExts][0]}` : 'No other apps for selection'}</span>
							</div>
						{/if}
					</div>
				</div>
			{/if}
		</div>
	{/if}

	{#if canReassign}
		<div class="reassign-surface">
			<div class="reassign-apps">
				<span class="reassign-from">{reassignSourceApp?.name ?? "app"}</span>
				<span class="reassign-arrow">&#x279C;</span>
				<span class="reassign-to">{reassignTargetApp?.name ?? "app"}</span>
			</div>
			<div class="reassign-exts">
				{#each [...selectedExts] as ext, i (ext)}
					<span class="reassign-ext-bubble" style="animation-delay: {i * 30}ms">.{ext}</span>
				{/each}
			</div>
			<button class="reassign-btn" onclick={doReassign}>
				Reassign
			</button>
		</div>
	{/if}

	<footer>
		<span>{summary[0]} apps</span>
		<span>{summary[1]} extensions</span>
	</footer>
</main>

<style>
	:global(body) {
		margin: 0;
		font-family: var(--font-ui);
		background: linear-gradient(145deg, #1e1e2e 0%, #1a1a2a 50%, #1c1c30 100%);
		color: var(--text-primary);
		-webkit-font-smoothing: antialiased;
		text-rendering: optimizeLegibility;
	}

	main {
		display: flex;
		flex-direction: column;
		height: 100vh;
		position: relative;
	}

	/* Noise texture overlay */
	main::before {
		content: '';
		position: fixed;
		inset: 0;
		pointer-events: none;
		z-index: 100;
		opacity: 0.03;
		background-image: url("data:image/svg+xml,%3Csvg viewBox='0 0 256 256' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.9' numOctaves='4' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)'/%3E%3C/svg%3E");
		background-repeat: repeat;
		background-size: 256px 256px;
	}

	footer {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 16px;
		padding: 8px 20px;
		background: linear-gradient(180deg, var(--ctp-crust) 0%, var(--bg-mantle) 100%);
		border-top: none;
		font-size: var(--font-sm);
		color: var(--text-muted);
		flex-shrink: 0;
		position: relative;
	}

	footer::before {
		content: '';
		position: absolute;
		top: 0;
		left: 5%;
		right: 5%;
		height: 1px;
		background: linear-gradient(90deg, transparent, var(--ctp-surface1), transparent);
	}

	/* Reassign surface */
	.reassign-surface {
		position: fixed;
		bottom: 40px;
		left: 50%;
		transform: translateX(-50%);
		background: rgba(49, 50, 68, 0.85);
		backdrop-filter: blur(16px);
		-webkit-backdrop-filter: blur(16px);
		border: 1px solid rgba(166, 227, 161, 0.15);
		border-radius: 16px;
		padding: 14px 28px 16px;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 14px;
		box-shadow:
			0 8px 40px rgba(0, 0, 0, 0.5),
			0 0 1px rgba(166, 227, 161, 0.2),
			inset 0 1px 0 rgba(255, 255, 255, 0.03);
		z-index: 10;
		white-space: nowrap;
		animation: surface-in 0.25s var(--ease-out-expo);
	}

	@keyframes surface-in {
		from {
			opacity: 0;
			transform: translateX(-50%) translateY(16px) scale(0.97);
			filter: blur(4px);
		}
		to {
			opacity: 1;
			transform: translateX(-50%) translateY(0) scale(1);
			filter: blur(0);
		}
	}

	.reassign-apps {
		display: flex;
		align-items: center;
		gap: 10px;
		font-size: var(--font-md);
	}

	.reassign-exts {
		display: flex;
		flex-wrap: wrap;
		justify-content: center;
		gap: 6px;
		max-width: 360px;
	}

	.reassign-ext-bubble {
		font-family: var(--font-mono);
		font-size: var(--font-sm);
		font-weight: 600;
		color: var(--accent-ext);
		background: var(--bg-crust);
		padding: 2px 8px;
		border-radius: 6px;
		animation: bubble-in 0.2s var(--ease-out-expo) both;
	}

	@keyframes bubble-in {
		from {
			opacity: 0;
			transform: scale(0.8) translateY(4px);
		}
		to {
			opacity: 1;
			transform: scale(1) translateY(0);
		}
	}

	.reassign-from {
		font-size: var(--font-md);
		font-weight: 500;
		color: var(--text-muted);
	}

	.reassign-to {
		font-size: var(--font-md);
		font-weight: 600;
		color: var(--text-primary);
	}

	.reassign-arrow {
		font-size: 14px;
		color: var(--accent-target);
	}

	.reassign-btn {
		background: var(--ctp-green);
		color: var(--ctp-crust);
		border: none;
		border-radius: 8px;
		padding: 8px 32px;
		font-weight: 600;
		font-size: var(--font-md);
		cursor: pointer;
		width: 100%;
		transition:
			background var(--duration-fast) ease,
			box-shadow var(--duration-normal) ease,
			transform 0.05s ease;
	}

	.reassign-btn:hover {
		background: #b5e8b2;
		box-shadow: 0 0 16px rgba(166, 227, 161, 0.3);
	}

	.reassign-btn:active {
		background: #93d990;
		transform: scale(0.98);
	}

	/* Loading */
	.loading {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		flex: 1;
		gap: 16px;
		color: var(--text-muted);
	}

	.loading-text {
		font-size: var(--font-xs);
		letter-spacing: var(--tracking-widest);
		text-transform: uppercase;
		color: var(--text-faint);
	}

	.loading-dots {
		display: flex;
		gap: 6px;
	}

	.loading-dots span {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: var(--accent-focus);
		animation: dot-pulse 1.2s ease-in-out infinite;
	}

	.loading-dots span:nth-child(2) { animation-delay: 0.15s; }
	.loading-dots span:nth-child(3) { animation-delay: 0.3s; }

	@keyframes dot-pulse {
		0%, 80%, 100% { opacity: 0.2; transform: scale(0.8); }
		40% { opacity: 1; transform: scale(1); }
	}

	/* Panels */
	.panels {
		display: flex;
		flex: 1;
		overflow: hidden;
	}

	.panel {
		display: flex;
		flex-direction: column;
		border-right: none;
		min-width: 0;
		position: relative;
	}

	/* Gradient dividers */
	.panel:not(:last-child)::after {
		content: '';
		position: absolute;
		right: 0;
		top: 10%;
		bottom: 10%;
		width: 1px;
		background: linear-gradient(
			180deg,
			transparent 0%,
			var(--ctp-surface2) 30%,
			var(--ctp-surface2) 70%,
			transparent 100%
		);
		z-index: 1;
	}

	.panel-focused .panel-header {
		border-bottom-color: var(--accent-focus);
	}

	.panel-focused .app-item.cursor:not(.active),
	.panel-focused .ext-item.cursor:not(.selected) {
		outline: 1px solid var(--accent-focus);
		outline-offset: -1px;
	}

	.apps-panel {
		flex: 0 0 220px;
		max-width: 260px;
	}

	.extensions-panel {
		flex: 1;
		min-width: 300px;
	}

	.targets-panel {
		flex: 0 0 240px;
		max-width: 280px;
	}

	.panel-header {
		padding: 10px 12px 8px;
		border-bottom: 1px solid var(--border);
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
		flex-shrink: 0;
		transition: border-color var(--duration-normal) ease;
	}

	.panel-header h2 {
		font-size: var(--font-xs);
		margin: 0;
		font-weight: 600;
		white-space: nowrap;
		letter-spacing: var(--tracking-widest);
		text-transform: uppercase;
		color: var(--text-muted);
	}

	.sort-toggle {
		padding: 2px 8px;
		font-size: var(--font-sm);
		font-weight: 600;
		min-width: 40px;
		position: relative;
	}

	.sort-toggle::after {
		content: '';
		position: absolute;
		bottom: -1px;
		left: 50%;
		transform: translateX(-50%);
		width: 12px;
		height: 2px;
		border-radius: 1px;
		background: var(--accent-focus);
		opacity: 0.6;
	}

	.any-toggle {
		padding: 2px 8px;
		font-size: var(--font-sm);
		font-weight: 600;
		color: var(--text-muted);
		border-color: var(--border);
		background: var(--bg-mantle);
		transition: color var(--duration-fast) ease, border-color var(--duration-fast) ease;
	}

	.any-toggle.any-active {
		color: var(--accent);
		border-color: var(--accent);
		background: var(--bg-mantle);
	}

	.panel-header input[type="text"] {
		flex: 1;
		padding: 4px 8px;
		border: 1px solid var(--border);
		border-radius: 4px;
		background: var(--bg-crust);
		color: var(--text-primary);
		font-size: var(--font-base);
		font-family: var(--font-ui);
		transition:
			border-color var(--duration-normal) ease,
			box-shadow var(--duration-normal) ease;
	}

	.panel-header input[type="text"]:focus {
		outline: none;
		border-color: var(--accent-focus);
		box-shadow: 0 0 0 2px rgba(203, 166, 247, 0.15);
	}

	.search-box {
		display: flex;
		align-items: center;
		flex: 1;
		position: relative;
	}

	.search-box input {
		width: 100%;
		padding-right: 24px;
	}

	.search-clear {
		position: absolute;
		right: 2px;
		padding: 0 4px;
		border: none;
		background: transparent;
		color: var(--text-muted);
		font-size: 16px;
		cursor: pointer;
		line-height: 1;
	}

	.search-clear:hover {
		color: var(--text-primary);
		background: transparent;
	}

	.panel-body {
		flex: 1;
		overflow-y: auto;
		padding: 4px 4px;
	}

	/* Custom scrollbars */
	.panel-body::-webkit-scrollbar {
		width: 6px;
	}

	.panel-body::-webkit-scrollbar-track {
		background: transparent;
	}

	.panel-body::-webkit-scrollbar-thumb {
		background: var(--ctp-surface1);
		border-radius: 3px;
	}

	.panel-body::-webkit-scrollbar-thumb:hover {
		background: var(--ctp-surface2);
	}

	button {
		padding: 4px 10px;
		border: 1px solid var(--border);
		border-radius: 4px;
		background: var(--bg-mantle);
		color: var(--text-primary);
		font-size: var(--font-base);
		font-family: var(--font-ui);
		cursor: pointer;
		transition:
			background var(--duration-fast) ease,
			transform 0.05s ease;
	}

	button:hover {
		background: var(--bg-surface0);
	}

	button:active {
		transform: scale(0.98);
	}

	.app-item {
		display: flex;
		align-items: center;
		justify-content: space-between;
		width: 100%;
		box-sizing: border-box;
		padding: 6px 10px;
		border: none;
		border-left: 2px solid transparent;
		border-radius: 6px;
		background: transparent;
		text-align: left;
		font-size: var(--font-md);
		cursor: pointer;
		outline: none;
		transition:
			background var(--duration-fast) ease,
			color var(--duration-fast) ease,
			border-color var(--duration-fast) ease;
	}

	.app-item:hover {
		background: var(--item-hover);
	}

	.app-item.active {
		background: var(--item-selected-app);
		color: var(--accent-source);
		border-left-color: var(--accent-source);
		box-shadow: inset 0 0 20px var(--glow-source);
	}

	.app-item.active .badge {
		color: var(--accent-source);
		background: transparent;
	}

	.app-item.owner-highlight:not(.active) {
		background: rgba(180, 190, 254, 0.08);
		color: var(--text-primary);
		border-left-color: rgba(180, 190, 254, 0.3);
	}

	.app-item.owner-highlight:not(.active) .badge {
		color: var(--badge-text);
		background: var(--badge-bg);
	}

	/* Fallback for combined active + owner-highlight */
	.app-item.active.owner-highlight {
		background: var(--item-selected-app);
		color: var(--accent-source);
		border-left-color: var(--accent-source);
	}

	.app-item.active.owner-highlight .badge {
		color: var(--accent-source);
		background: transparent;
	}

	.badge {
		font-size: var(--font-sm);
		font-variant-numeric: tabular-nums;
		background: var(--badge-bg);
		padding: 1px 6px;
		border-radius: 8px;
		color: var(--badge-text);
		min-width: 24px;
		text-align: center;
	}

	.ext-item {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 5px 10px;
		margin: 0 0;
		border-radius: 6px;
		cursor: pointer;
		user-select: none;
		font-size: var(--font-md);
		outline: none;
		transition:
			background var(--duration-fast) ease,
			color var(--duration-fast) ease;
	}

	.ext-item:hover {
		background: var(--item-hover);
	}

	.ext-item.selected {
		background: var(--item-selected-ext);
		color: var(--text-primary);
		animation: select-flash 0.3s ease;
	}

	@keyframes select-flash {
		0% { background: rgba(116, 199, 236, 0.3); }
		100% { background: var(--item-selected-ext); }
	}

	.ext-item.selected .ext-name {
		color: var(--accent-ext);
	}

	.ext-item.selected .ext-desc {
		color: var(--text-secondary);
	}

	.ext-item.selected .ext-default {
		color: var(--text-secondary);
	}

	.ext-item.dimmed .ext-name,
	.ext-item.dimmed .ext-desc,
	.ext-item.dimmed .ext-default {
		color: var(--ctp-overlay0);
	}

	.ext-name {
		font-family: var(--font-mono);
		font-weight: 500;
		letter-spacing: 0.02em;
		font-size: var(--font-base);
		color: var(--accent);
	}

	.ext-desc {
		color: var(--text-muted);
		font-size: var(--font-base);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.ext-default {
		margin-left: auto;
		color: var(--text-muted);
		font-size: var(--font-sm);
		white-space: nowrap;
	}

	/* Empty & loading states */
	.empty {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		padding: 40px 20px;
		gap: 8px;
		color: var(--text-faint);
	}

	.empty-icon {
		font-size: 24px;
		opacity: 0.4;
	}

	.empty-label {
		font-size: var(--font-base);
		letter-spacing: 0.02em;
	}
</style>
