<script lang="ts">
	import { onMount } from "svelte";
	import {
		getApps,
		getExtensionsForApp,
		getCandidateTargets,
		getEligibleExtensions,
		reassignExtensions,
		getAppsForExtension,
		getSummary,
	} from "./lib/api";
	import type { App, Extension } from "./lib/types";
	import { resolveAction } from "./lib/keymap";
	import type { Action } from "./lib/keymap";

	let apps: App[] = $state([]);
	let allExtensions: Extension[] = $state([]);
	let extensions: Extension[] = $state([]);
	let candidateTargets: App[] = $state([]);
	let eligibleExts: Set<string> = $state(new Set());
	let selectedExts: Set<string> = $state(new Set());
	let summary: [number, number] = $state([0, 0]);

	let extFilter = $state("");
	let selectedSourceId: number | null = $state(null);
	let selectedTargetId: number | null = $state(null);

	let appSort = $state<"alpha" | "ext_count">("ext_count");
	let loading = $state(true);

	type Panel = "apps" | "extensions" | "targets";
	let focusedPanel: Panel = $state("apps");
	let appCursor = $state(0);
	let extCursor = $state(0);
	let targetCursor = $state(0);

	let extFilterInputEl: HTMLInputElement | undefined = $state(undefined);
	let panelBodyEls: Record<Panel, HTMLElement | undefined> = $state({
		apps: undefined,
		extensions: undefined,
		targets: undefined,
	});

	let sortedApps = $derived(
		appSort === "alpha"
			? apps
			: [...apps].sort((a, b) => b.ext_count - a.ext_count),
	);

	let activeExtensions = $derived(extFilter ? allExtensions : extensions);

	let filteredExtensions = $derived(
		extFilter
			? activeExtensions.filter(
					(e) =>
						e.ext.includes(extFilter.toLowerCase()) ||
						e.description
							.toLowerCase()
							.includes(extFilter.toLowerCase()),
				)
			: activeExtensions,
	);

	let sortedExtensions = $derived(
		selectedTargetId !== null
			? [...filteredExtensions].sort((a, b) => {
					const aEligible = eligibleExts.has(a.ext) ? 0 : 1;
					const bEligible = eligibleExts.has(b.ext) ? 0 : 1;
					if (aEligible !== bEligible) return aEligible - bEligible;
					return a.ext.localeCompare(b.ext);
				})
			: filteredExtensions,
	);

	let greyedExts = $derived(
		selectedTargetId !== null
			? new Set(
					extensions
						.filter((e) => !eligibleExts.has(e.ext))
						.map((e) => e.ext),
				)
			: new Set<string>(),
	);

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
			appCursor = 0;
			await selectSource(sortedApps[0].id);
		}
		loading = false;
	}

	onMount(() => {
		init();
	});

	async function selectSource(appId: number | null) {
		selectedSourceId = appId;
		selectedTargetId = null;
		selectedExts = new Set();
		eligibleExts = new Set();
		extFilter = "";
		extCursor = 0;
		targetCursor = 0;

		if (appId !== null) {
			const [exts, targets] = await Promise.all([
				getExtensionsForApp(appId),
				getCandidateTargets(appId),
			]);
			extensions = exts;
			candidateTargets = targets;
		} else {
			extensions = await getExtensionsForApp();
			candidateTargets = [];
		}
	}

	async function selectTarget(appId: number | null) {
		selectedTargetId = appId;
		selectedExts = new Set();

		if (appId !== null && selectedSourceId !== null) {
			const eligible = await getEligibleExtensions(
				selectedSourceId,
				appId,
			);
			eligibleExts = new Set(eligible);
		} else {
			eligibleExts = new Set();
		}
	}

	function toggleExt(ext: string, event: MouseEvent) {
		if (greyedExts.has(ext)) return;
		const next = new Set(selectedExts);
		if (event.metaKey || event.ctrlKey) {
			if (next.has(ext)) next.delete(ext);
			else next.add(ext);
		} else if (event.shiftKey) {
			const allExts = sortedExtensions
				.filter((e) => !greyedExts.has(e.ext))
				.map((e) => e.ext);
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
			next.clear();
			next.add(ext);
		}
		selectedExts = next;
	}

	function selectAllEligible() {
		if (selectedTargetId === null) {
			selectedExts = new Set(extensions.map((e) => e.ext));
		} else {
			selectedExts = new Set(eligibleExts);
		}
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
		if (panel === "targets") return candidateTargets.length;
		return 0;
	}

	function cursorFor(panel: Panel): number {
		if (panel === "apps") return appCursor;
		if (panel === "extensions") return extCursor;
		return targetCursor;
	}

	function setCursor(panel: Panel, val: number) {
		if (panel === "apps") appCursor = val;
		else if (panel === "extensions") extCursor = val;
		else targetCursor = val;
	}

	function clampCursor(panel: Panel) {
		const len = panelListLength(panel);
		const cur = cursorFor(panel);
		if (len === 0) {
			setCursor(panel, 0);
		} else if (cur >= len) {
			setCursor(panel, len - 1);
		}
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

	async function selectAtCursor() {
		if (focusedPanel === "apps") {
			if (appCursor < sortedApps.length) {
				await selectSource(sortedApps[appCursor].id);
			}
		} else if (focusedPanel === "targets") {
			if (targetCursor < candidateTargets.length) {
				await selectTarget(candidateTargets[targetCursor].id);
			}
		}
	}

	async function handleAction(action: Action) {
		const panels: Panel[] = ["apps", "extensions", "targets"];
		const panelIdx = panels.indexOf(focusedPanel);

		switch (action) {
			case "focus_left":
				if (panelIdx > 0) focusedPanel = panels[panelIdx - 1];
				clampCursor(focusedPanel);
				break;
			case "focus_right":
				if (panelIdx < panels.length - 1)
					focusedPanel = panels[panelIdx + 1];
				clampCursor(focusedPanel);
				break;

			case "move_down": {
				const len = panelListLength(focusedPanel);
				const cur = cursorFor(focusedPanel);
				if (cur < len - 1) {
					setCursor(focusedPanel, cur + 1);
					scrollCursorIntoView(focusedPanel);
					if (
						focusedPanel === "apps" ||
						focusedPanel === "targets"
					)
						await selectAtCursor();
				}
				break;
			}
			case "move_up": {
				const cur = cursorFor(focusedPanel);
				if (cur > 0) {
					setCursor(focusedPanel, cur - 1);
					scrollCursorIntoView(focusedPanel);
					if (
						focusedPanel === "apps" ||
						focusedPanel === "targets"
					)
						await selectAtCursor();
				}
				break;
			}
			case "move_top":
				setCursor(focusedPanel, 0);
				scrollCursorIntoView(focusedPanel);
				if (focusedPanel === "apps" || focusedPanel === "targets")
					await selectAtCursor();
				break;
			case "move_bottom": {
				const len = panelListLength(focusedPanel);
				if (len > 0) setCursor(focusedPanel, len - 1);
				scrollCursorIntoView(focusedPanel);
				if (focusedPanel === "apps" || focusedPanel === "targets")
					await selectAtCursor();
				break;
			}

			case "select":
				await selectAtCursor();
				break;

			case "toggle_select":
				if (focusedPanel === "extensions") {
					const ext = sortedExtensions[extCursor]?.ext;
					if (ext && !greyedExts.has(ext)) {
						const next = new Set(selectedExts);
						if (next.has(ext)) next.delete(ext);
						else next.add(ext);
						selectedExts = next;
					}
				}
				break;

			case "extend_down":
				if (focusedPanel === "extensions") {
					const ext = sortedExtensions[extCursor]?.ext;
					if (ext && !greyedExts.has(ext)) {
						const next = new Set(selectedExts);
						next.add(ext);
						selectedExts = next;
					}
					if (extCursor < sortedExtensions.length - 1) {
						extCursor++;
						const nextExt = sortedExtensions[extCursor]?.ext;
						if (nextExt && !greyedExts.has(nextExt)) {
							const next2 = new Set(selectedExts);
							next2.add(nextExt);
							selectedExts = next2;
						}
						scrollCursorIntoView("extensions");
					}
				}
				break;

			case "extend_up":
				if (focusedPanel === "extensions") {
					const ext = sortedExtensions[extCursor]?.ext;
					if (ext && !greyedExts.has(ext)) {
						const next = new Set(selectedExts);
						next.add(ext);
						selectedExts = next;
					}
					if (extCursor > 0) {
						extCursor--;
						const nextExt = sortedExtensions[extCursor]?.ext;
						if (nextExt && !greyedExts.has(nextExt)) {
							const next2 = new Set(selectedExts);
							next2.add(nextExt);
							selectedExts = next2;
						}
						scrollCursorIntoView("extensions");
					}
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
		<div class="loading">Loading...</div>
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
						onclick={() =>
							(appSort =
								appSort === "alpha" ? "ext_count" : "alpha")}
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
							onclick={() => {
								focusedPanel = "apps";
								appCursor = i;
								selectSource(app.id);
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
					<input
						type="text"
						placeholder="Search all extensions..."
						bind:value={extFilter}
						bind:this={extFilterInputEl}
						oninput={() => { extCursor = 0; }}
					/>
					{#if selectedTargetId !== null}
						<button onclick={selectAllEligible}>
							Select All ({eligibleExts.size})
						</button>
					{/if}
				</div>
				<div class="panel-body" bind:this={panelBodyEls.extensions}>
					{#each sortedExtensions as ext, i (ext.ext)}
						<div
							class="ext-item"
							class:selected={selectedExts.has(ext.ext)}
							class:cursor={extCursor === i}
							class:greyed={greyedExts.has(ext.ext)}
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
						<div class="empty">No extensions</div>
					{/if}
				</div>
				{#if selectedExts.size > 0 && selectedTargetId !== null}
					{@const targetApp = candidateTargets.find(
						(a) => a.id === selectedTargetId,
					)}
					<div class="reassign-bar">
						<button class="reassign-btn" onclick={doReassign}>
							Reassign {selectedExts.size} extension{selectedExts.size >
							1
								? "s"
								: ""} to {targetApp?.name ?? "app"}
						</button>
					</div>
				{/if}
			</div>

			<!-- Right: Target Apps -->
			{#if selectedSourceId !== null}
				<div
					class="panel targets-panel"
					class:panel-focused={focusedPanel === "targets"}
				>
					<div class="panel-header">
						<h2>Reassign To</h2>
					</div>
					<div
						class="panel-body"
						bind:this={panelBodyEls.targets}
					>
						{#each candidateTargets as app, i (app.id)}
							<button
								class="app-item"
								class:cursor={targetCursor === i}
								class:active={selectedTargetId === app.id}
								onclick={() => {
									focusedPanel = "targets";
									targetCursor = i;
									selectTarget(app.id);
								}}
							>
								<span class="app-name">{app.name}</span>
								<span class="badge"
									>{app.ext_count}</span
								>
							</button>
						{/each}
						{#if candidateTargets.length === 0}
							<div class="empty">
								No other apps handle these extensions
							</div>
						{/if}
					</div>
				</div>
			{/if}
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
		font-family:
			-apple-system,
			BlinkMacSystemFont,
			"Segoe UI",
			Roboto,
			sans-serif;
		background: var(--bg-base);
		color: var(--text-primary);
	}

	main {
		display: flex;
		flex-direction: column;
		height: 100vh;
	}

	footer {
		display: flex;
		align-items: center;
		gap: 16px;
		padding: 6px 16px;
		background: var(--bg-mantle);
		border-top: 1px solid var(--border);
		font-size: 12px;
		color: var(--text-muted);
		flex-shrink: 0;
	}

	.loading {
		display: flex;
		align-items: center;
		justify-content: center;
		flex: 1;
		color: var(--text-muted);
	}

	.panels {
		display: flex;
		flex: 1;
		overflow: hidden;
	}

	.panel {
		display: flex;
		flex-direction: column;
		border-right: 1px solid var(--border);
		min-width: 0;
	}

	.panel-focused .panel-header {
		border-bottom-color: var(--accent);
	}

	.panel-focused .app-item.cursor,
	.panel-focused .ext-item.cursor,
	.panel-focused .cursor {
		background: var(--ctp-blue);
		color: var(--ctp-crust);
	}

	.panel-focused .cursor .badge,
	.panel-focused .cursor .ext-name,
	.panel-focused .cursor .ext-desc,
	.panel-focused .cursor .ext-default {
		color: inherit;
	}

	.app-item.cursor,
	.ext-item.cursor,
	.cursor {
		background: var(--ctp-surface1);
		color: var(--text-primary);
	}

	.cursor .badge,
	.cursor .ext-name,
	.cursor .ext-desc,
	.cursor .ext-default {
		color: inherit;
	}

	.cursor .badge {
		background: transparent;
	}

	.panel:last-child {
		border-right: none;
	}

	.apps-panel {
		flex: 0 0 auto;
		min-width: 240px;
	}

	.extensions-panel {
		flex: 1;
	}

	.targets-panel {
		flex: 0 0 auto;
		min-width: 240px;
	}

	.panel-header {
		padding: 8px 12px;
		border-bottom: 1px solid var(--border);
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
		flex-shrink: 0;
	}

	.panel-header h2 {
		font-size: 13px;
		margin: 0;
		font-weight: 600;
		white-space: nowrap;
	}

	.sort-toggle {
		padding: 2px 8px;
		font-size: 11px;
		font-weight: 600;
		min-width: 40px;
	}

	.panel-header input[type="text"] {
		flex: 1;
		padding: 4px 8px;
		border: 1px solid var(--border);
		border-radius: 4px;
		background: var(--bg-crust);
		color: var(--text-primary);
		font-size: 12px;
	}

	.panel-body {
		flex: 1;
		overflow-y: auto;
		padding: 4px 0;
	}

	button {
		padding: 4px 10px;
		border: 1px solid var(--border);
		border-radius: 4px;
		background: var(--bg-mantle);
		color: var(--text-primary);
		font-size: 12px;
		cursor: pointer;
	}

	button:hover {
		background: var(--bg-surface0);
	}

	.app-item {
		display: flex;
		align-items: center;
		justify-content: space-between;
		width: 100%;
		box-sizing: border-box;
		padding: 6px 12px;
		border: none;
		border-radius: 0;
		background: transparent;
		text-align: left;
		font-size: 13px;
		cursor: pointer;
	}

	.app-item:hover {
		background: var(--item-hover);
	}

	.app-item.active {
		background: var(--ctp-surface0);
	}

	.badge {
		font-size: 11px;
		background: var(--badge-bg);
		padding: 1px 6px;
		border-radius: 8px;
		color: var(--badge-text);
	}

	.ext-item {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 5px 12px;
		cursor: pointer;
		user-select: none;
		font-size: 13px;
	}

	.ext-item:hover {
		background: var(--item-hover);
	}

	.ext-item.selected {
		background: var(--ctp-surface1);
	}

	.panel-focused .ext-item.selected {
		background: var(--ctp-blue);
		color: var(--ctp-crust);
	}

	.panel-focused .ext-item.selected .ext-name,
	.panel-focused .ext-item.selected .ext-desc,
	.panel-focused .ext-item.selected .ext-default {
		color: inherit;
	}

	.ext-item.greyed {
		opacity: 0.3;
		pointer-events: none;
	}

	.ext-name {
		font-family: "SF Mono", "Fira Code", monospace;
		font-weight: 500;
		color: var(--accent);
	}

	.ext-desc {
		color: var(--text-muted);
		font-size: 12px;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.ext-default {
		margin-left: auto;
		color: var(--text-muted);
		font-size: 11px;
		white-space: nowrap;
	}

	.empty {
		padding: 20px;
		text-align: center;
		color: var(--text-faint);
		font-size: 13px;
	}

	.reassign-bar {
		padding: 8px 12px;
		border-top: 1px solid var(--border);
		background: var(--bg-mantle);
		flex-shrink: 0;
	}

	.reassign-btn {
		width: 100%;
		padding: 8px 16px;
		background: var(--ctp-green);
		color: var(--ctp-crust);
		border: none;
		border-radius: 6px;
		font-size: 13px;
		font-weight: 600;
		cursor: pointer;
	}

	.reassign-btn:hover {
		opacity: 0.9;
	}
</style>
