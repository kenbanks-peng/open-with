import { invoke } from "@tauri-apps/api/core";
import type { App, Group, GroupDetail } from "./types";

export async function getApps(filter?: string): Promise<App[]> {
	return invoke("get_apps", { filter: filter || null });
}

export async function getGroups(
	appFilterId?: number,
	assignedOnly: boolean = false,
): Promise<Group[]> {
	return invoke("get_groups", {
		appFilterId: appFilterId ?? null,
		assignedOnly,
	});
}

export async function getGroupDetail(
	groupId: number | null,
): Promise<GroupDetail> {
	return invoke("get_group_detail", { groupId });
}

export async function validateMove(
	exts: string[],
	targetGroupId: number,
): Promise<boolean> {
	return invoke("validate_move", { exts, targetGroupId });
}

export async function moveExtensions(
	exts: string[],
	targetGroupId: number | null,
): Promise<void> {
	return invoke("move_extensions", { exts, targetGroupId });
}

export async function createGroup(name: string): Promise<Group> {
	return invoke("create_group", { name });
}

export async function renameGroup(
	groupId: number,
	name: string,
): Promise<void> {
	return invoke("rename_group", { groupId, name });
}

export async function deleteGroup(groupId: number): Promise<void> {
	return invoke("delete_group", { groupId });
}

export async function assignAppToGroup(
	groupId: number,
	appId: number | null,
): Promise<void> {
	return invoke("assign_app_to_group", { groupId, appId });
}

export async function breakoutGroup(groupId: number): Promise<number> {
	return invoke("breakout_group", { groupId });
}

export async function getAppsForExtension(ext: string): Promise<App[]> {
	return invoke("get_apps_for_extension", { ext });
}

export async function getSummary(): Promise<[number, number, number]> {
	return invoke("get_summary");
}
