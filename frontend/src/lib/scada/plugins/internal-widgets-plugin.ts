import { internalWidgetDeclarations } from "$lib/scada/plugins/internal-widgets/widgets";
import type { ScadaInternalPlugin } from "$lib/scada/plugins/types";

export const internalWidgetsPlugin: ScadaInternalPlugin = {
  id: "lirays.internal.widgets",
  version: "1.0.0",
  displayName: "Internal Widgets",
  contributes: {
    ui: {
      widgets: internalWidgetDeclarations,
    },
  },
};
