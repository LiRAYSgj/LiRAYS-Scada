import type { InternalWidgetDeclaration } from "$lib/scada/plugins/types";
import { fanWidget } from "./fan/fan.widget";
import { labelWidget } from "./label/label.widget";
import { lightWidget } from "./light/light.widget";
import { onoffWidget } from "./onoff/onoff.widget";
import { pumpWidget } from "./pump/pump.widget";
import { sliderWidget } from "./slider/slider.widget";
import { tankWidget } from "./tank/tank.widget";
import { typedInputWidget } from "./typed-input/typed-input.widget";
import { valveWidget } from "./valve/valve.widget";

export const internalWidgetDeclarations: InternalWidgetDeclaration[] = [
  tankWidget,
  pumpWidget,
  valveWidget,
  fanWidget,
  sliderWidget,
  typedInputWidget,
  onoffWidget,
  lightWidget,
  labelWidget,
];
