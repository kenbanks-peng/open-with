export interface App {
	id: number;
	name: string;
	path: string;
	ext_count: number;
}

export interface Extension {
	ext: string;
	group_id: number | null;
	description: string;
}

export interface Group {
	id: number;
	name: string;
	assigned_app_id: number | null;
	assigned_app_name: string | null;
	ext_count: number;
}

export interface GroupDetail {
	group: Group;
	extensions: Extension[];
	common_apps: App[];
}
