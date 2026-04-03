<script lang="ts">
	import { onDestroy } from "svelte";
	import { toast } from "svelte-sonner";
	import { Toaster } from "$lib/components/ui/sonner";
	import { snackbarStore } from "$lib/stores/snackbar";

	const unsubscribe = snackbarStore.subscribe((entry) => {
		if (!entry) return;
		const opts = { duration: entry.duration };
		if (entry.type === "success") {
			toast.success(entry.message, opts);
		} else if (entry.type === "warning") {
			toast.warning(entry.message, opts);
		} else {
			toast.error(entry.message, opts);
		}
		snackbarStore.hide();
	});

	onDestroy(() => {
		unsubscribe();
	});
</script>

<Toaster />
