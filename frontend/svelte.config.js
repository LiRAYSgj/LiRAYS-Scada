import adapter from "@sveltejs/adapter-static";

export default {
  kit: {
    adapter: adapter({
      pages: "build",
      assets: "build",
      strict: false,
      // Keep prerendered pages while allowing backend-served/dynamic routes (e.g. /api/*, /views/[id]).
      // No fallback avoids overwriting prerendered root index.html.
    }),
  },
};
