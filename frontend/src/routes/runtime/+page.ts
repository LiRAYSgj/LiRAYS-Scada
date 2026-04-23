import { redirect } from "@sveltejs/kit";
import type { PageLoad } from "./$types";
import { getEntryPointView } from "$lib/features/views/api/views-api";

export const load: PageLoad = async () => {
  const view = await getEntryPointView().catch(() => null);
  if (!view) {
    throw redirect(307, "/runtime/no-entry");
  }

  throw redirect(307, `/runtime/${view.id}`);
};
