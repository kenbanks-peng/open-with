<script lang="ts">
	import { onMount } from "svelte";
	import {
		getApps,
		getGroups,
		getGroupDetail,
		getAppsForExtension,
		validateMove,
		moveExtensions,
		createGroup,
		renameGroup,
		deleteGroup,
		assignAppToGroup,
		breakoutGroup,
		getSummary,
	} from "./lib/api";
	import type { App, Group, GroupDetail } from "./lib/types";
	import { resolveAction } from "./lib/keymap";
	import type { Action } from "./lib/keymap";

	let apps: App[] = $state([]);
	let groups: Group[] = $state([]);
	let groupDetail: GroupDetail | null = $state(null);
	let summary: [number, number, number] = $state([0, 0, 0]);

	let appFilter = $state("");
	let selectedAppId: number | null = $state(null);
	let selectedGroupId: number | null = $state(null);
	let selectedExts: Set<string> = $state(new Set());
	let editingGroupId: number | null = $state(null);
	let editingName = $state("");

	let appSort: "alpha" | "ext_count" = $state("ext_count");
	let groupFilterMode: "eligible" | "assigned" = $state("assigned");

	let extApps: App[] = $state([]);

	let dragExts: string[] = $state([]);
	let dropValid: Record<number, boolean> = $state({});
	let loading = $state(true);

	type Panel = "apps" | "groups" | "extensions";
	let focusedPanel: Panel = $state("apps");
	let appCursor = $state(0);
	let groupCursor = $state(0);
	let extCursor = $state(0);

	let filterInputEl: HTMLInputElement | undefined = $state(undefined);
	let panelBodyEls: Record<Panel, HTMLElement | undefined> = $state({
		apps: undefined,
		groups: undefined,
		extensions: undefined,
	});

	let sortedApps = $derived(
		appSort === "alpha"
			? apps
			: [...apps].sort((a, b) => b.ext_count - a.ext_count),
	);

	let sortedGroups = $derived(
		[...groups].sort((a, b) => {
			const aAssigned = selectedAppId !== null && a.assigned_app_id === selectedAppId;
			const bAssigned = selectedAppId !== null && b.assigned_app_id === selectedAppId;
			if (aAssigned !== bAssigned) return aAssigned ? -1 : 1;
			return a.name.localeCompare(b.name);
		}),
	);

	async function refresh() {
		const assignedOnly =
			selectedAppId !== null && groupFilterMode === "assigned";
		const [a, g, s] = await Promise.all([
			getApps(appFilter || undefined),
			getGroups(selectedAppId ?? undefined, assignedOnly),
			getSummary(),
		]);
		apps = a;
		groups = g;
		summary = s;
		if (selectedGroupId !== null) {
			groupDetail = await getGroupDetail(selectedGroupId);
		}
	}

	async function init() {
		loading = true;
		await refresh();
		if (sortedApps.length > 0) {
			appCursor = 1;
			await selectApp(sortedApps[0].id);
		}
		loading = false;
	}

	onMount(() => {
		init();
	});

	async function onAppFilterInput() {
		apps = await getApps(appFilter || undefined);
	}

	async function selectApp(appId: number | null) {
		selectedAppId = appId;
		const assignedOnly = selectedAppId !== null && groupFilterMode === "assigned";
		groups = await getGroups(appId ?? undefined, assignedOnly);
		selectedExts = new Set();
		if (sortedGroups.length > 0) {
			await selectGroup(sortedGroups[0].id);
		} else {
			selectedGroupId = null;
			groupDetail = null;
		}
	}

	async function setGroupFilterMode(mode: "eligible" | "assigned") {
		groupFilterMode = mode;
		const assignedOnly = mode === "assigned";
		groups = await getGroups(selectedAppId ?? undefined, assignedOnly);
		selectedGroupId = null;
		groupDetail = null;
		selectedExts = new Set();
	}

	async function refreshExtApps() {
		if (groupDetail && extCursor < groupDetail.extensions.length) {
			extApps = await getAppsForExtension(groupDetail.extensions[extCursor].ext);
		} else {
			extApps = [];
		}
	}

	async function selectGroup(groupId: number) {
		selectedGroupId = groupId;
		groupDetail = await getGroupDetail(groupId);
		selectedExts = new Set();
		extCursor = 0;
		await refreshExtApps();
	}

	function toggleExt(ext: string, event: MouseEvent) {
		const next = new Set(selectedExts);
		if (event.metaKey || event.ctrlKey) {
			if (next.has(ext)) next.delete(ext);
			else next.add(ext);
		} else if (event.shiftKey && groupDetail) {
			const allExts = groupDetail.extensions.map((e) => e.ext);
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

	function onDragStart(event: DragEvent) {
		const exts = [...selectedExts];
		if (exts.length === 0) return;
		dragExts = exts;
		event.dataTransfer!.effectAllowed = "move";
		event.dataTransfer!.setData("text/plain", exts.join(","));
	}

	async function onDragOverGroup(event: DragEvent, groupId: number) {
		if (dragExts.length === 0 || groupId === selectedGroupId) return;
		event.preventDefault();
		if (groupId === -1) {
			dropValid = { ...dropValid, [groupId]: true };
			return;
		}
		if (dropValid[groupId] === undefined) {
			const valid = await validateMove(dragExts, groupId);
			dropValid = { ...dropValid, [groupId]: valid };
		}
		if (dropValid[groupId]) {
			event.dataTransfer!.dropEffect = "move";
		}
	}

	async function onDropGroup(event: DragEvent, groupId: number) {
		event.preventDefault();
		if (dragExts.length === 0) return;
		if (groupId !== -1 && !dropValid[groupId]) return;

		const targetId = groupId === -1 ? null : groupId;
		await moveExtensions(dragExts, targetId);
		dragExts = [];
		dropValid = {};
		await refresh();
		if (selectedGroupId !== null) {
			groupDetail = await getGroupDetail(selectedGroupId);
		}
	}

	function onDragEnd() {
		dragExts = [];
		dropValid = {};
	}

	async function onCreateGroup() {
		const name = "New Group";
		const group = await createGroup(name);
		await refresh();
		editingGroupId = group.id;
		editingName = name;
	}

	async function onDeleteGroup(groupId: number) {
		await deleteGroup(groupId);
		if (selectedGroupId === groupId) {
			selectedGroupId = null;
			groupDetail = null;
		}
		await refresh();
	}

	function startRename(group: Group) {
		editingGroupId = group.id;
		editingName = group.name;
	}

	async function finishRename() {
		if (editingGroupId !== null && editingName.trim()) {
			await renameGroup(editingGroupId, editingName.trim());
			editingGroupId = null;
			await refresh();
		}
	}

	function onRenameKeydown(event: KeyboardEvent) {
		if (event.key === "Enter") finishRename();
		if (event.key === "Escape") {
			editingGroupId = null;
		}
	}

	async function onBreakout(groupId: number) {
		const created = await breakoutGroup(groupId);
		if (created > 0) {
			selectedGroupId = null;
			groupDetail = null;
			await refresh();
		}
	}

	async function onAssignApp(groupId: number, appId: number | null) {
		await assignAppToGroup(groupId, appId);
		await refresh();
		if (selectedGroupId === groupId) {
			groupDetail = await getGroupDetail(groupId);
		}
	}

	function panelListLength(panel: Panel): number {
		if (panel === "apps") return sortedApps.length + 1; // +1 for "All"
		if (panel === "groups") return sortedGroups.length;
		if (panel === "extensions") return groupDetail?.extensions.length ?? 0;
		return 0;
	}

	function cursorFor(panel: Panel): number {
		if (panel === "apps") return appCursor;
		if (panel === "groups") return groupCursor;
		return extCursor;
	}

	function setCursor(panel: Panel, val: number) {
		if (panel === "apps") appCursor = val;
		else if (panel === "groups") groupCursor = val;
		else extCursor = val;
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
		body.scrollBy({ top: direction * (body.clientHeight / 2), behavior: "smooth" });
	}

	async function selectAtCursor() {
		if (focusedPanel === "apps") {
			const idx = appCursor;
			if (idx === 0) await selectApp(null);
			else if (idx - 1 < sortedApps.length) await selectApp(sortedApps[idx - 1].id);
			groupCursor = 0;
		} else if (focusedPanel === "groups") {
			if (groupCursor < sortedGroups.length) await selectGroup(sortedGroups[groupCursor].id);
			extCursor = 0;
		}
	}

	async function handleAction(action: Action) {
		const panels: Panel[] = ["apps", "groups", "extensions"];
		const panelIdx = panels.indexOf(focusedPanel);

		switch (action) {
			case "focus_left":
				if (panelIdx > 0) focusedPanel = panels[panelIdx - 1];
				clampCursor(focusedPanel);
				break;
			case "focus_right":
				if (panelIdx < panels.length - 1) focusedPanel = panels[panelIdx + 1];
				clampCursor(focusedPanel);
				if (focusedPanel === "extensions") await refreshExtApps();
				break;

			case "move_down": {
				const len = panelListLength(focusedPanel);
				const cur = cursorFor(focusedPanel);
				if (cur < len - 1) {
					setCursor(focusedPanel, cur + 1);
					scrollCursorIntoView(focusedPanel);
					if (focusedPanel === "apps" || focusedPanel === "groups") await selectAtCursor();
					else if (focusedPanel === "extensions") await refreshExtApps();
				}
				break;
			}
			case "move_up": {
				const cur = cursorFor(focusedPanel);
				if (cur > 0) {
					setCursor(focusedPanel, cur - 1);
					scrollCursorIntoView(focusedPanel);
					if (focusedPanel === "apps" || focusedPanel === "groups") await selectAtCursor();
					else if (focusedPanel === "extensions") await refreshExtApps();
				}
				break;
			}
			case "move_top":
				setCursor(focusedPanel, 0);
				scrollCursorIntoView(focusedPanel);
				if (focusedPanel === "apps" || focusedPanel === "groups") await selectAtCursor();
				else if (focusedPanel === "extensions") await refreshExtApps();
				break;
			case "move_bottom": {
				const len = panelListLength(focusedPanel);
				if (len > 0) setCursor(focusedPanel, len - 1);
				scrollCursorIntoView(focusedPanel);
				if (focusedPanel === "apps" || focusedPanel === "groups") await selectAtCursor();
				else if (focusedPanel === "extensions") await refreshExtApps();
				break;
			}

			case "select":
				await selectAtCursor();
				break;

			case "toggle_select":
				if (focusedPanel === "extensions" && groupDetail) {
					const ext = groupDetail.extensions[extCursor]?.ext;
					if (ext) {
						const next = new Set(selectedExts);
						if (next.has(ext)) next.delete(ext);
						else next.add(ext);
						selectedExts = next;
					}
				}
				break;

			case "extend_down":
				if (focusedPanel === "extensions" && groupDetail) {
					const ext = groupDetail.extensions[extCursor]?.ext;
					if (ext) {
						const next = new Set(selectedExts);
						next.add(ext);
						selectedExts = next;
					}
					const len = groupDetail.extensions.length;
					if (extCursor < len - 1) {
						extCursor++;
						const nextExt = groupDetail.extensions[extCursor]?.ext;
						if (nextExt) {
							const next2 = new Set(selectedExts);
							next2.add(nextExt);
							selectedExts = next2;
						}
						scrollCursorIntoView("extensions");
						await refreshExtApps();
					}
				}
				break;

			case "extend_up":
				if (focusedPanel === "extensions" && groupDetail) {
					const ext = groupDetail.extensions[extCursor]?.ext;
					if (ext) {
						const next = new Set(selectedExts);
						next.add(ext);
						selectedExts = next;
					}
					if (extCursor > 0) {
						extCursor--;
						const nextExt = groupDetail.extensions[extCursor]?.ext;
						if (nextExt) {
							const next2 = new Set(selectedExts);
							next2.add(nextExt);
							selectedExts = next2;
						}
						scrollCursorIntoView("extensions");
						await refreshExtApps();
					}
				}
				break;

			case "search":
				filterInputEl?.focus();
				break;

			case "escape":
				if (editingGroupId !== null) {
					editingGroupId = null;
				} else if (document.activeElement === filterInputEl) {
					appFilter = "";
					filterInputEl?.blur();
					await onAppFilterInput();
				} else {
					selectedExts = new Set();
				}
				break;

			case "rename":
				if (focusedPanel === "groups" && groupCursor < sortedGroups.length) {
					const g = sortedGroups[groupCursor];
					if (g.id !== -1) startRename(g);
				}
				break;

			case "delete":
				if (focusedPanel === "groups" && groupCursor < sortedGroups.length) {
					const g = sortedGroups[groupCursor];
					if (g.id !== -1) await onDeleteGroup(g.id);
					clampCursor("groups");
				}
				break;

			case "new_group":
				await onCreateGroup();
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
			<!-- Left: Apps -->
			<div class="panel apps-panel" class:panel-focused={focusedPanel === 'apps'}>
				<div class="panel-header">
					<h2>Apps</h2>
					<button
						class="sort-toggle"
						onclick={() => appSort = appSort === "alpha" ? "ext_count" : "alpha"}
						title={appSort === "alpha" ? "Sorted A-Z; click to sort by extension count" : "Sorted by extension count; click to sort A-Z"}
					>
						{appSort === "alpha" ? "A-Z" : "#Ext"}
					</button>
					<input
						type="text"
						placeholder="Filter apps..."
						bind:value={appFilter}
						bind:this={filterInputEl}
						oninput={onAppFilterInput}
					/>
				</div>
				<div class="panel-body" bind:this={panelBodyEls.apps}>
					<button
						class="app-item"
						class:cursor={appCursor === 0}
						onclick={() => { focusedPanel = 'apps'; selectApp(null); appCursor = 0; }}
					>
						All
					</button>
					{#each sortedApps as app, i (app.id)}
						<button
							class="app-item"
							class:cursor={appCursor === i + 1}
							onclick={() => { focusedPanel = 'apps'; selectApp(app.id); appCursor = i + 1; }}
						>
							<span class="app-name">{app.name}</span>
							<span class="badge">{app.ext_count}</span>
						</button>
					{/each}
				</div>
			</div>

			<!-- Middle: Groups -->
			<div class="panel groups-panel" class:panel-focused={focusedPanel === 'groups'}>
				<div class="panel-header">
					<h2>Groups</h2>
					{#if selectedAppId !== null}
						<button
							class="toggle-btn"
							onclick={() => setGroupFilterMode(groupFilterMode === "eligible" ? "assigned" : "eligible")}
						>
							{groupFilterMode === "eligible" ? "Eligible" : "Assigned"}
						</button>
					{/if}
					<button onclick={onCreateGroup}>+ New Group</button>
				</div>
				<div class="panel-body" bind:this={panelBodyEls.groups}>
					{#each sortedGroups as group, i (group.id)}
						<div
							class="group-item"
							class:cursor={groupCursor === i}
							class:drop-valid={dragExts.length > 0 && dropValid[group.id] === true}
							class:drop-invalid={dragExts.length > 0 && dropValid[group.id] === false}
							role="option"
							aria-selected={selectedGroupId === group.id}
							tabindex="0"
							onclick={() => { focusedPanel = 'groups'; selectGroup(group.id); groupCursor = i; }}
							onkeydown={(e: KeyboardEvent) => e.key === 'Enter' && selectGroup(group.id)}
							ondragover={(e: DragEvent) => onDragOverGroup(e, group.id)}
							ondrop={(e: DragEvent) => onDropGroup(e, group.id)}
						>
							{#if editingGroupId === group.id}
								<input
									type="text"
									class="rename-input"
									bind:value={editingName}
									onblur={finishRename}
									onkeydown={onRenameKeydown}
								/>
							{:else}
								<div class="group-info">
									<span
										class="group-name"
										class:muted={selectedAppId !== null && group.assigned_app_id !== selectedAppId}
										role="button"
										tabindex="0"
										ondblclick={() => group.id !== -1 && startRename(group)}
										onkeydown={(e: KeyboardEvent) => e.key === 'F2' && group.id !== -1 && startRename(group)}
									>
										{group.name}
									</span>
									<span class="group-meta">
										{#if group.assigned_app_name}
											<span class="assigned" class:muted={selectedAppId !== null && group.assigned_app_id !== selectedAppId}>{group.assigned_app_name}</span>
										{:else if group.id !== -1}
											<span class="unassigned">unassigned</span>
										{/if}
									</span>
								</div>
								<div class="group-actions">
									<span class="badge">{group.ext_count}</span>
									{#if group.id !== -1}
										<button
											class="delete-btn"
											onclick={(e: MouseEvent) => { e.stopPropagation(); onDeleteGroup(group.id); }}
											title="Delete group"
										>
											&times;
										</button>
									{/if}
								</div>
							{/if}
						</div>
					{/each}
				</div>
			</div>

			<!-- Right: Extensions -->
			<div class="panel extensions-panel" class:panel-focused={focusedPanel === 'extensions'}>
				{#if groupDetail}
					<div class="panel-header">
						<h2>{groupDetail.group.name}</h2>
						{#if focusedPanel === 'extensions' && groupDetail.group.id !== -1 && extApps.length > 0}
							<select
								value={groupDetail.group.assigned_app_id ?? ""}
								onchange={(e: Event) => {
									const val = (e.target as HTMLSelectElement).value;
									onAssignApp(groupDetail!.group.id, val ? Number(val) : null);
								}}
							>
								<option value="">-- Assign app --</option>
								{#each extApps as app (app.id)}
									<option value={app.id}>{app.name}</option>
								{/each}
							</select>
						{/if}
						{#if groupDetail.group.id !== -1 && groupDetail.extensions.length > 1}
							<button onclick={() => onBreakout(groupDetail!.group.id)} title="Split into sub-groups by app compatibility">Breakout</button>
						{/if}
					</div>
					<div class="panel-body" bind:this={panelBodyEls.extensions}>
						{#each groupDetail.extensions as ext, i (ext.ext)}
							<div
								class="ext-item"
								class:selected={selectedExts.has(ext.ext)}
								class:cursor={extCursor === i}
								draggable="true"
								role="option"
								aria-selected={selectedExts.has(ext.ext)}
								tabindex="0"
								onclick={(e: MouseEvent) => { focusedPanel = 'extensions'; toggleExt(ext.ext, e); extCursor = i; refreshExtApps(); }}
								onkeydown={(e: KeyboardEvent) => e.key === 'Enter' && toggleExt(ext.ext, e as unknown as MouseEvent)}
								ondragstart={onDragStart}
								ondragend={onDragEnd}
							>
								<span class="ext-name">.{ext.ext}</span>
								{#if ext.description}
									<span class="ext-desc">{ext.description}</span>
								{/if}
							</div>
						{/each}
						{#if groupDetail.extensions.length === 0}
							<div class="empty">No extensions in this group</div>
						{/if}
					</div>
				{:else}
					<div class="panel-header">
						<h2>Extensions</h2>
					</div>
					<div class="panel-body">
						<div class="empty">Select a group to view extensions</div>
					</div>
				{/if}
			</div>
		</div>
	{/if}

	<footer>
		<span>{summary[0]} apps</span>
		<span>{summary[1]} groups</span>
		<span>{summary[2]} extensions</span>
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
	.panel-focused .group-item.cursor,
	.panel-focused .ext-item.cursor,
	.panel-focused .cursor {
		background: var(--ctp-blue);
		color: var(--ctp-crust);
	}

	.panel-focused .cursor .group-name,
	.panel-focused .cursor .assigned,
	.panel-focused .cursor .unassigned,
	.panel-focused .cursor .badge,
	.panel-focused .cursor .delete-btn,
	.panel-focused .cursor .ext-name,
	.panel-focused .cursor .ext-desc {
		color: inherit;
	}

	.group-item.cursor,
	.ext-item.cursor,
	.app-item.cursor,
	.cursor {
		background: var(--ctp-surface1);
		color: var(--text-primary);
	}

	.cursor .group-name,
	.cursor .assigned,
	.cursor .unassigned,
	.cursor .badge,
	.cursor .delete-btn,
	.cursor .ext-name,
	.cursor .ext-desc {
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

	.groups-panel {
		flex: 0 0 auto;
		min-width: 300px;
	}

	.extensions-panel {
		flex: 1;
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
	}

	.toggle-btn {
		padding: 2px 8px;
		font-size: 11px;
		font-weight: 600;
		min-width: 64px;
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

	.panel-header select {
		padding: 4px 6px;
		border: 1px solid var(--border);
		border-radius: 4px;
		background: var(--bg-crust);
		color: var(--text-primary);
		font-size: 12px;
		max-width: 160px;
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

	.app-item,
	.group-item {
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

	.app-item:hover,
	.group-item:hover {
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
	.panel-focused .ext-item.selected .ext-desc {
		color: inherit;
	}

	.group-item.drop-valid {
		background: var(--drop-valid-bg);
		outline: 2px solid var(--drop-valid-border);
		outline-offset: -2px;
	}

	.group-item.drop-invalid {
		opacity: 0.5;
	}

	.group-info {
		display: flex;
		flex-direction: column;
		gap: 2px;
		min-width: 0;
		overflow: hidden;
	}

	.group-name {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.group-meta {
		font-size: 11px;
	}

	.muted {
		color: var(--text-muted);
	}

	.assigned {
		color: var(--success);
	}

	.unassigned {
		color: var(--text-muted);
		font-style: italic;
	}

	.group-actions {
		display: flex;
		align-items: center;
		gap: 4px;
		flex-shrink: 0;
	}

	.badge {
		font-size: 11px;
		background: var(--badge-bg);
		padding: 1px 6px;
		border-radius: 8px;
		color: var(--badge-text);
	}

	.delete-btn {
		padding: 0 4px;
		border: none;
		background: transparent;
		color: var(--text-muted);
		font-size: 16px;
		cursor: pointer;
	}

	.delete-btn:hover {
		color: var(--danger);
	}

	.rename-input {
		width: 100%;
		padding: 2px 6px;
		border: 1px solid var(--accent);
		border-radius: 3px;
		background: var(--bg-crust);
		color: var(--text-primary);
		font-size: 13px;
	}

	.ext-item {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 5px 12px;
		cursor: grab;
		user-select: none;
		font-size: 13px;
	}

	.ext-item:hover {
		background: var(--item-hover);
	}

	.ext-item.selected {
		background: var(--item-selected);
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

	.empty {
		padding: 20px;
		text-align: center;
		color: var(--text-faint);
		font-size: 13px;
	}
</style>
