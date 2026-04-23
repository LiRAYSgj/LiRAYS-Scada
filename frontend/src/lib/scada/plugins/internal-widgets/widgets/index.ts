import type { InternalWidgetDeclaration } from "$lib/scada/plugins/types";
import { alarmWidget } from "./alarm/alarm.widget";
import { fanWidget } from "./fan/fan.widget";
import { gaugeWidget } from "./gauge/gauge.widget";
import { labelWidget } from "./label/label.widget";
import { levelIndicatorWidget } from "./level-indicator/level-indicator.widget";
import { lightWidget } from "./light/light.widget";
import { onoffWidget } from "./onoff/onoff.widget";
import { pumpWidget } from "./pump/pump.widget";
import { pushButtonWidget } from "./push-button/push-button.widget";
import { rectangleWidget } from "./rectangle/rectangle.widget";
import { sevenSegmentWidget } from "./seven-segment/seven-segment.widget";
import { sliderWidget } from "./slider/slider.widget";
import { tankWidget } from "./tank/tank.widget";
import { textWidget } from "./text/text.widget";
import { thermometerWidget } from "./thermometer/thermometer.widget";
import { typedInputWidget } from "./typed-input/typed-input.widget";
import { valveWidget } from "./valve/valve.widget";
import { markdownWidget } from "./markdown/markdown.widget";

export const internalWidgetDeclarations: InternalWidgetDeclaration[] = [
  pushButtonWidget,
  tankWidget,
  thermometerWidget,
  gaugeWidget,
  sevenSegmentWidget,
  alarmWidget,
  pumpWidget,
  valveWidget,
  fanWidget,
  sliderWidget,
  typedInputWidget,
  onoffWidget,
  lightWidget,
  labelWidget,
  textWidget,
  levelIndicatorWidget,
  rectangleWidget,
  markdownWidget,
];
