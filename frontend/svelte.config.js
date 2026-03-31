import adapter from "@sveltejs/adapter-static";

export default {
  kit: {
    adapter: adapter({
      pages: "build",
      assets: "build",
      // No fallback: every route is prerendered (`+layout.ts`). Avoids overwriting prerendered `index.html`.
    }),
  },
};
