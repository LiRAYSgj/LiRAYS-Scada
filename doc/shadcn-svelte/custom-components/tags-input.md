# Tags Input

Chip-based text input for entering and managing multiple values (for example, **Allowed options** in variable metadata forms).

## Installation

This is a project-local custom component and is already available in this repository.

Source files:

- `frontend/src/lib/components/ui/tags-input/tags-input.svelte`
- `frontend/src/lib/components/ui/tags-input/tags-input-tag.svelte`
- `frontend/src/lib/components/ui/tags-input/tags-input-suggestion.svelte`
- `frontend/src/lib/components/ui/tags-input/index.ts`
- `frontend/src/lib/components/ui/tags-input/types.ts`

## Usage

```svelte
<script lang="ts">
  import { TagsInput } from "$lib/components/ui/tags-input";

  let options = $state<string[]>([]);
</script>

<TagsInput
  class="w-full text-sm"
  bind:value={options}
  placeholder="Type an option and press comma or Enter"
  commitSeparators={[","]}
/>
```

## API

`TagsInput` accepts standard input attributes plus the props below:

- `value?: string[]` - Selected tags.
- `validate?: (val: string, tags: string[]) => string | undefined` - Validation/normalization hook for each committed value.
- `onValueChange?: (value: string[]) => void` - Callback when tags change.
- `suggestions?: string[]` - Optional suggestion list.
- `filterSuggestions?: (inputValue: string, suggestions: string[]) => string[]` - Custom filtering strategy.
- `restrictToSuggestions?: boolean` - If `true`, only suggestion values can be committed.
- `commitSeparators?: string[]` - Keys that commit current input (default includes comma).

## Examples

### Allowed options (Text variables)

```svelte
<script lang="ts">
  import { TagsInput } from "$lib/components/ui/tags-input";

  let addOptionTags = $state<string[]>([]);
</script>

<div class="space-y-1">
  <label class="text-xs text-muted-foreground" for="add-options">Allowed options</label>
  <TagsInput
    id="add-options"
    class="w-full text-sm"
    bind:value={addOptionTags}
    commitSeparators={[","]}
    placeholder="Type an option and press comma or Enter"
  />
</div>
```

### With suggestions + restrict mode

```svelte
<script lang="ts">
  import { TagsInput } from "$lib/components/ui/tags-input";

  const regions = ["America", "Europa", "Asia"];
  let selected = $state<string[]>(["America"]);
</script>

<TagsInput
  class="w-full text-sm"
  bind:value={selected}
  suggestions={regions}
  restrictToSuggestions={true}
  placeholder="Choose region(s)"
/>
```

### Custom validation (no duplicates, max 10 chars)

```svelte
<script lang="ts">
  import { TagsInput } from "$lib/components/ui/tags-input";

  let tags = $state<string[]>([]);

  const validateTag = (val: string, current: string[]) => {
    const normalized = val.trim();
    if (!normalized) return undefined;
    if (normalized.length > 10) return undefined;
    if (current.includes(normalized)) return undefined;
    return normalized;
  };
</script>

<TagsInput bind:value={tags} validate={validateTag} placeholder="max 10 chars" />
```

