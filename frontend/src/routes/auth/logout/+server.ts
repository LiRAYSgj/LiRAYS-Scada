import type { RequestHandler } from "./$types";
import { forwardAuthRequest } from "$lib/server/auth-proxy";

export const GET: RequestHandler = async (event) => {
  return forwardAuthRequest(event, "/auth/logout", "GET");
};
