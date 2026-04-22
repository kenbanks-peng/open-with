export type Action =
	| "focus_left"
	| "focus_right"
	| "move_up"
	| "move_down"
	| "move_top"
	| "move_bottom"
	| "select"
	| "toggle_select"
	| "extend_up"
	| "extend_down"
	| "select_all"
	| "search"
	| "escape"
	| "reassign"
	| "undo"
	| "scroll_half_down"
	| "scroll_half_up";

interface KeyCombo {
	key: string;
	ctrl?: boolean;
	shift?: boolean;
}

const keymap: { combo: KeyCombo; action: Action }[] = [
	// Panel focus
	{ combo: { key: "h" }, action: "focus_left" },
	{ combo: { key: "ArrowLeft" }, action: "focus_left" },
	{ combo: { key: "h", ctrl: true }, action: "focus_left" },
	{ combo: { key: "l" }, action: "focus_right" },
	{ combo: { key: "ArrowRight" }, action: "focus_right" },
	{ combo: { key: "l", ctrl: true }, action: "focus_right" },

	// List navigation
	{ combo: { key: "j" }, action: "move_down" },
	{ combo: { key: "ArrowDown" }, action: "move_down" },
	{ combo: { key: "k" }, action: "move_up" },
	{ combo: { key: "ArrowUp" }, action: "move_up" },
	{ combo: { key: "g" }, action: "move_top" },
	{ combo: { key: "G", shift: true }, action: "move_bottom" },

	// Selection
	{ combo: { key: "Enter" }, action: "select" },
	{ combo: { key: " " }, action: "toggle_select" },
	{ combo: { key: "J", shift: true }, action: "extend_down" },
	{ combo: { key: "ArrowDown", shift: true }, action: "extend_down" },
	{ combo: { key: "K", shift: true }, action: "extend_up" },
	{ combo: { key: "ArrowUp", shift: true }, action: "extend_up" },
	{ combo: { key: "a" }, action: "select_all" },

	// Actions
	{ combo: { key: "/" }, action: "search" },
	{ combo: { key: "Escape" }, action: "escape" },
	{ combo: { key: "r" }, action: "reassign" },
	{ combo: { key: "z", ctrl: true }, action: "undo" },

	// Scrolling
	{ combo: { key: "d", ctrl: true }, action: "scroll_half_down" },
	{ combo: { key: "u", ctrl: true }, action: "scroll_half_up" },
];

function matchCombo(e: KeyboardEvent, combo: KeyCombo): boolean {
	if (combo.ctrl && !e.ctrlKey && !e.metaKey) return false;
	if (!combo.ctrl && (e.ctrlKey || e.metaKey)) return false;
	if (combo.shift && !e.shiftKey) return false;
	if (!combo.shift && e.shiftKey) return false;
	return e.key === combo.key;
}

export function resolveAction(e: KeyboardEvent): Action | null {
	for (const entry of keymap) {
		if (matchCombo(e, entry.combo)) return entry.action;
	}
	return null;
}
