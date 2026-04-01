import { z } from "zod";
import { ItemType, VarDataType } from "$lib/proto/namespace/enums";
import {
  sanitizeCsvList,
  sanitizeIdentifierLike,
  sanitizeText,
} from "./sanitize";

const optionalNumber = z.preprocess((value) => {
  if (value === "" || value === null || value === undefined) return undefined;
  return value;
}, z.coerce.number().finite().optional());

const optionalInteger = z.preprocess((value) => {
  if (value === "" || value === null || value === undefined) return undefined;
  return value;
}, z.coerce.number().int().nonnegative().optional());

export const addTreeItemSchema = z
  .object({
    name: z
      .string()
      .transform((v) => sanitizeIdentifierLike(v, 128))
      .refine((v) => v.length > 0, "Name is required")
      .refine((v) => !v.includes("/"), 'Name cannot contain "/"'),
    kind: z.coerce.number().int(),
    dataType: z.coerce.number().int(),
    unit: z
      .string()
      .optional()
      .transform((v) => sanitizeIdentifierLike(v ?? "", 32)),
    min: optionalNumber,
    max: optionalNumber,
    options: z.string().optional().default(""),
    maxLen: optionalInteger,
  })
  .superRefine((value, ctx) => {
    if (
      value.kind !== ItemType.ITEM_TYPE_FOLDER &&
      value.kind !== ItemType.ITEM_TYPE_VARIABLE
    ) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ["kind"],
        message: "Invalid node type",
      });
    }

    if (
      value.dataType !== VarDataType.VAR_DATA_TYPE_INTEGER &&
      value.dataType !== VarDataType.VAR_DATA_TYPE_FLOAT &&
      value.dataType !== VarDataType.VAR_DATA_TYPE_TEXT &&
      value.dataType !== VarDataType.VAR_DATA_TYPE_BOOLEAN
    ) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ["dataType"],
        message: "Invalid data type",
      });
    }

    if (
      value.kind === ItemType.ITEM_TYPE_VARIABLE &&
      (value.dataType === VarDataType.VAR_DATA_TYPE_INTEGER ||
        value.dataType === VarDataType.VAR_DATA_TYPE_FLOAT)
    ) {
    }
  });

export const editTreeMetaSchema = z.object({
  varId: z
    .string()
    .transform((v) => sanitizeIdentifierLike(v, 200))
    .refine((v) => v.length > 0, "Variable ID is required"),
  dataType: z.string(),
  unit: z
    .string()
    .optional()
    .transform((v) => sanitizeIdentifierLike(v ?? "", 32)),
  min: optionalNumber,
  max: optionalNumber,
  options: z.string().optional().default(""),
  maxLen: optionalInteger,
});

export function toCreateItemPayload(
  parsed: z.infer<typeof addTreeItemSchema>,
): {
  name: string;
  itemType: ItemType;
  varType: VarDataType | undefined;
  unit?: string;
  min?: number;
  max?: number;
  options?: string[];
  maxLen?: number;
} {
  const kind = parsed.kind as ItemType;
  const dataType = parsed.dataType as VarDataType;
  const unit = parsed.unit ? sanitizeText(parsed.unit, 32) : "";
  const min = parsed.min;
  const max = parsed.max;
  const maxLen = parsed.maxLen;

  return {
    name: sanitizeIdentifierLike(parsed.name, 128),
    itemType: kind,
    varType: kind === ItemType.ITEM_TYPE_VARIABLE ? dataType : undefined,
    unit: unit || undefined,
    min:
      kind === ItemType.ITEM_TYPE_VARIABLE &&
      (dataType === VarDataType.VAR_DATA_TYPE_INTEGER ||
        dataType === VarDataType.VAR_DATA_TYPE_FLOAT)
        ? min
        : undefined,
    max:
      kind === ItemType.ITEM_TYPE_VARIABLE &&
      (dataType === VarDataType.VAR_DATA_TYPE_INTEGER ||
        dataType === VarDataType.VAR_DATA_TYPE_FLOAT)
        ? max
        : undefined,
    options:
      kind === ItemType.ITEM_TYPE_VARIABLE &&
      dataType === VarDataType.VAR_DATA_TYPE_TEXT
        ? sanitizeCsvList(parsed.options)
        : undefined,
    maxLen:
      kind === ItemType.ITEM_TYPE_VARIABLE &&
      dataType === VarDataType.VAR_DATA_TYPE_TEXT
        ? maxLen
        : undefined,
  };
}

export function toEditMetaPayload(parsed: z.infer<typeof editTreeMetaSchema>): {
  varId: string;
  unit?: string;
  min?: number;
  max?: number;
  options?: string[];
  maxLen?: number;
} {
  const unit = parsed.unit ? sanitizeText(parsed.unit, 32) : "";
  const min = parsed.min;
  const max = parsed.max;
  const maxLen = parsed.maxLen;

  return {
    varId: parsed.varId,
    unit: unit || undefined,
    min:
      parsed.dataType === "VAR_DATA_TYPE_INTEGER" ||
      parsed.dataType === "VAR_DATA_TYPE_FLOAT"
        ? min
        : undefined,
    max:
      parsed.dataType === "VAR_DATA_TYPE_INTEGER" ||
      parsed.dataType === "VAR_DATA_TYPE_FLOAT"
        ? max
        : undefined,
    options:
      parsed.dataType === "VAR_DATA_TYPE_TEXT"
        ? sanitizeCsvList(parsed.options)
        : undefined,
    maxLen: parsed.dataType === "VAR_DATA_TYPE_TEXT" ? maxLen : undefined,
  };
}
