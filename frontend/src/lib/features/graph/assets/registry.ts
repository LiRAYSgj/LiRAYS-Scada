import TankAsset from "./components/TankAsset.svelte";
import PumpAsset from "./components/PumpAsset.svelte";
import ValveAsset from "./components/ValveAsset.svelte";
import FanAsset from "./components/FanAsset.svelte";
import SliderAsset from "./components/SliderAsset.svelte";
import TypedInputAsset from "./components/TypedInputAsset.svelte";
import OnOffInputAsset from "./components/OnOffInputAsset.svelte";
import LightIndicatorAsset from "./components/LightIndicatorAsset.svelte";
import LabelAsset from "./components/LabelAsset.svelte";
import { PlantAssetKind, type GraphAssetDefinition } from "./types";

const DEFAULT_PLANT_ASSET_DEFINITIONS: GraphAssetDefinition[] = [
  { name: PlantAssetKind.TANK, label: "Tank", component: TankAsset },
  { name: PlantAssetKind.PUMP, label: "Pump", component: PumpAsset },
  { name: PlantAssetKind.VALVE, label: "Valve", component: ValveAsset },
  { name: PlantAssetKind.FAN, label: "Fan", component: FanAsset },
  { name: PlantAssetKind.SLIDER, label: "Slider", component: SliderAsset },
  {
    name: PlantAssetKind.TYPED_INPUT,
    label: "Typed Input",
    component: TypedInputAsset,
  },
  {
    name: PlantAssetKind.ONOFF,
    label: "On/Off Input",
    component: OnOffInputAsset,
  },
  {
    name: PlantAssetKind.LIGHT,
    label: "Light Indicator",
    component: LightIndicatorAsset,
  },
  { name: PlantAssetKind.LABEL, label: "Label", component: LabelAsset },
];

const definitionByKind = new Map<PlantAssetKind, GraphAssetDefinition>();
const orderedDefinitions: GraphAssetDefinition[] = [];

function upsertAssetDefinition(definition: GraphAssetDefinition): void {
  const existingIndex = orderedDefinitions.findIndex(
    (item) => item.name === definition.name,
  );
  if (existingIndex >= 0) {
    orderedDefinitions[existingIndex] = definition;
  } else {
    orderedDefinitions.push(definition);
  }
  definitionByKind.set(definition.name, definition);
}

for (const definition of DEFAULT_PLANT_ASSET_DEFINITIONS) {
  upsertAssetDefinition(definition);
}

export function registerAsset(definition: GraphAssetDefinition): void {
  upsertAssetDefinition(definition);
}

export function getRegisteredAssetDefinitions(): GraphAssetDefinition[] {
  return [...orderedDefinitions];
}

export const PLANT_ASSET_DEFINITIONS: GraphAssetDefinition[] =
  orderedDefinitions;

export function resolveAssetDefinition(
  kind: PlantAssetKind,
): GraphAssetDefinition {
  return definitionByKind.get(kind) ?? orderedDefinitions[0];
}
