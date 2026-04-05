import type { RequestHandler } from "./$types";
import { forwardAuthRequest } from "$lib/server/auth-proxy";

export const POST: RequestHandler = async (event) => {
  return forwardAuthRequest(event, "/auth/setup", "POST");
};
