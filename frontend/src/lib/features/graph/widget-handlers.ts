import type { TagScalarValue } from "$lib/core/ws/types";
import type { WidgetInteractionEventName } from "$lib/features/graph/assets/types";

export interface WidgetRuntimeHandlers {
  onWriteValue?: (value: TagScalarValue) => void;
  onWriteBindingValue?: (
    bindingKey: string,
    value: TagScalarValue,
    tagId?: string,
  ) => void;
  onOpenBindingConfig?: (event: MouseEvent) => void;
  onWidgetEvent?: (
    eventName: WidgetInteractionEventName,
    payload?: unknown,
    event?: MouseEvent,
  ) => void;
}

const handlersBySymbolId = new Map<string, WidgetRuntimeHandlers>();

export function registerWidgetHandlers(
  symbolId: string,
  handlers: WidgetRuntimeHandlers,
): void {
  handlersBySymbolId.set(symbolId, handlers);
}

export function getWidgetHandlers(
  symbolId: string | undefined,
): WidgetRuntimeHandlers | undefined {
  if (!symbolId) {
    return undefined;
  }
  return handlersBySymbolId.get(symbolId);
}

export function unregisterWidgetHandlers(symbolId: string): void {
  handlersBySymbolId.delete(symbolId);
}

export function clearWidgetHandlers(): void {
  handlersBySymbolId.clear();
}
