# Number Field

Number input with embedded custom `-` / `+` stepper buttons (used for Min / Max / Max length controls in variable dialogs).

## Installation

This is a project-local custom component and is already available in this repository.

Source files:

- `frontend/src/lib/components/ui/number-field/number-field.svelte`
- `frontend/src/lib/components/ui/number-field/index.ts`

## Usage

```svelte
<script lang="ts">
  import { NumberField } from "$lib/components/ui/number-field";

  let value = $state<number | undefined>(undefined);
</script>

<NumberField bind:value class="w-full" placeholder="e.g. 0" />
```

## API

`NumberField` accepts standard input attributes (except `type`, `value`, `files`) plus:

- `value?: number`
- `step?: number | "any"` (default: `1`)
- `min?: number`
- `max?: number`
- `class?: string`
- `onValueChange?: (value: number | undefined) => void`

Behavior notes:

- Built-in spinner controls are hidden.
- Embedded `-` / `+` buttons update by `step`.
- Values are clamped to `min` / `max` when set.

## Examples

### Min / Max pair (float/integer variables)

```svelte
<script lang="ts">
  import { NumberField } from "$lib/components/ui/number-field";

  let min = $state<number | undefined>(undefined);
  let max = $state<number | undefined>(undefined);
</script>

<div class="grid grid-cols-2 gap-2">
  <div class="space-y-1">
    <label class="text-xs text-muted-foreground" for="add-min">Min</label>
    <NumberField
      id="add-min"
      step="any"
      class="w-full"
      bind:value={min}
      placeholder="e.g. 0"
    />
  </div>
  <div class="space-y-1">
    <label class="text-xs text-muted-foreground" for="add-max">Max</label>
    <NumberField
      id="add-max"
      step="any"
      class="w-full"
      bind:value={max}
      placeholder="e.g. 100"
    />
  </div>
</div>
```

### Max length (text variables)

```svelte
<script lang="ts">
  import { NumberField } from "$lib/components/ui/number-field";

  let maxLen = $state<number | undefined>(undefined);
</script>

<div class="space-y-1">
  <label class="text-xs text-muted-foreground" for="add-maxlen">Max length</label>
  <NumberField
    id="add-maxlen"
    step={1}
    min={0}
    class="w-full"
    bind:value={maxLen}
    placeholder="e.g. 32"
  />
</div>
```

### Callback on value updates

```svelte
<script lang="ts">
  import { NumberField } from "$lib/components/ui/number-field";

  let quantity = $state<number | undefined>(1);
  const handleChange = (next: number | undefined) => {
    console.log("NumberField value:", next);
  };
</script>

<NumberField bind:value={quantity} step={1} min={0} max={999} onValueChange={handleChange} />
```

