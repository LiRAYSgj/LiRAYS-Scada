import adapter from "@sveltejs/adapter-static";

export default {
  kit: {
    paths: {
      // Use absolute asset URLs (/_app/...) so deep links like /views/:id don't resolve
      // module imports relative to the current route segment.
      relative: false,
    },
    adapter: adapter({
      pages: "build",
      assets: "build",
      strict: false,
      // Keep prerendered pages while allowing backend-served/dynamic routes (e.g. /api/*, /views/[id]).
      // No fallback avoids overwriting prerendered root index.html.
    }),
  },
};
