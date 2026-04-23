export type ViewsColumnId = "view" | "updated" | "entry" | "actions";

export interface ViewsTableColumn {
  id: ViewsColumnId;
  title: string;
  className?: string;
}

export const VIEWS_TABLE_COLUMNS: ViewsTableColumn[] = [
  { id: "view", title: "View", className: "w-[46%]" },
  { id: "updated", title: "Updated", className: "w-[24%]" },
  { id: "entry", title: "Entry", className: "w-[14%]" },
  { id: "actions", title: "Actions", className: "w-[16%] text-right" },
];
