export interface App {
	id: number;
	name: string;
	path: string;
	ext_count: number;
}

export interface Extension {
	ext: string;
	description: string;
	default_app_name: string | null;
}
