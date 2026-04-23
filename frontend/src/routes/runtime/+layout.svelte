<script lang="ts">
  import { goto } from "$app/navigation";
  import type { LayoutData } from "./$types";
  import { themeStore } from "$lib/stores/theme";
  import PageToolbar from "$lib/features/workspace/components/PageToolbar.svelte";

  const theme = themeStore;
  let { children, data }: { children: import("svelte").Snippet; data: LayoutData } =
    $props();
</script>

<main class="flex h-dvh w-full flex-col gap-3 overflow-hidden bg-background p-4">
  <PageToolbar
    theme={$theme ?? "light"}
    workspaceMode="runtime"
    authEnabled={data.authEnabled}
    username={data.username}
    showWorkspaceModeSwitch={data.role !== "operator"}
    onSelectWorkspaceMode={() => {
      if (data.role !== "operator") {
        void goto("/views");
      }
    }}
    onToggleTheme={() => themeStore.update((current) => (current === "dark" ? "light" : "dark"))}
  />

  <div class="min-h-0 flex-1 overflow-hidden rounded-md border border-border bg-card">
    {@render children()}
  </div>
</main>
