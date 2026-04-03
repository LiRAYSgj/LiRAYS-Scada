import type { MenuOption, MenuOptionIcon } from "../context-menu";

export interface ResolvedMenuOption {
	id: string;
	label: string;
	icon?: MenuOptionIcon;
	separator?: boolean;
	disabled?: boolean;
	onSelect?: MenuOption["onSelect"];
	children?: ResolvedMenuOption[];
}
