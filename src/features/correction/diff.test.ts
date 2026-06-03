import { describe, expect, it } from "vitest";
import { diffWords } from "./diff";

describe("diffWords", () => {
  it("marks nothing changed for identical text", () => {
    const segs = diffWords("ala ma kota", "ala ma kota");
    expect(segs.every((s) => !s.changed)).toBe(true);
    expect(segs.map((s) => s.text).join("")).toBe("ala ma kota");
  });

  it("flags an inserted/changed word", () => {
    const segs = diffWords("ala ma kota", "ala ma psa");
    const changed = segs.filter((s) => s.changed).map((s) => s.text.trim());
    expect(changed).toContain("psa");
    // reconstruction equals the corrected text
    expect(segs.map((s) => s.text).join("")).toBe("ala ma psa");
  });

  it("does not flag pure whitespace", () => {
    const segs = diffWords("a b", "a  b");
    expect(segs.filter((s) => s.changed && /^\s+$/.test(s.text))).toHaveLength(0);
  });
});
