import { invoke } from "@tauri-apps/api/core";
import type { App, Extension } from "./types";

export async function getApps(filter?: string): Promise<App[]> {
	return invoke("get_apps", { filter: filter || null });
}

export async function getExtensionsForApp(
	appId?: number,
): Promise<Extension[]> {
	return invoke("get_extensions_for_app", { appId: appId ?? null });
}

export async function getCandidateTargets(sourceAppId: number): Promise<App[]> {
	return invoke("get_candidate_targets", { sourceAppId });
}

export async function getEligibleExtensions(
	sourceAppId: number,
	targetAppId: number,
): Promise<string[]> {
	return invoke("get_eligible_extensions", { sourceAppId, targetAppId });
}

export async function getExtensionTargetCounts(
	sourceAppId: number,
): Promise<[string, number][]> {
	return invoke("get_extension_target_counts", { sourceAppId });
}

export async function reassignExtensions(
	exts: string[],
	targetAppId: number,
): Promise<void> {
	return invoke("reassign_extensions", { exts, targetAppId });
}

export async function getAppsForExtension(ext: string): Promise<App[]> {
	return invoke("get_apps_for_extension", { ext });
}

export async function getAppsForExtensions(
	exts: string[],
	excludeAppId?: number,
): Promise<App[]> {
	return invoke("get_apps_for_extensions", {
		exts,
		excludeAppId: excludeAppId ?? null,
	});
}

export async function getSummary(): Promise<[number, number]> {
	return invoke("get_summary");
}
