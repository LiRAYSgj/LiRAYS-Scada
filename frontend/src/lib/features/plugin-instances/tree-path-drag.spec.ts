import { describe, expect, it } from "vitest";
import { folderPattern } from "./tree-path-drag";

describe("folderPattern", () => {
  it("matches everything below a folder, including the root", () => {
    expect(folderPattern("/plant/line1")).toBe("/plant/line1/**");
    expect(folderPattern("/plant/")).toBe("/plant/**");
    expect(folderPattern("/")).toBe("/**");
  });
});
