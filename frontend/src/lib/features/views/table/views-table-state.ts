export interface ViewsTableState {
  pageIndex: number;
  pageSize: number;
  sortBy: "name" | "updated_at" | "is_entry_point";
  sortDirection: "asc" | "desc";
}

export const DEFAULT_VIEWS_TABLE_STATE: ViewsTableState = {
  pageIndex: 0,
  pageSize: 10,
  sortBy: "updated_at",
  sortDirection: "desc",
};
