/** Word-level diff for highlighting corrections (parity with Python difflib). */

export interface DiffSegment {
  text: string;
  changed: boolean;
}

function tokenize(s: string): string[] {
  // keep whitespace as its own tokens so we can rebuild the string exactly
  return s.split(/(\s+)/).filter((t) => t.length > 0);
}

/**
 * Returns segments of `corrected` with `changed=true` for tokens that are not
 * part of the longest common subsequence with `original` (i.e. inserted or
 * altered words). Used to underline what the model changed.
 */
export function diffWords(original: string, corrected: string): DiffSegment[] {
  const a = tokenize(original);
  const b = tokenize(corrected);
  const n = a.length;
  const m = b.length;

  // LCS table
  const dp: number[][] = Array.from({ length: n + 1 }, () =>
    new Array<number>(m + 1).fill(0),
  );
  for (let i = n - 1; i >= 0; i--) {
    for (let j = m - 1; j >= 0; j--) {
      dp[i][j] =
        a[i] === b[j]
          ? dp[i + 1][j + 1] + 1
          : Math.max(dp[i + 1][j], dp[i][j + 1]);
    }
  }

  // Walk b, marking tokens not in the LCS as changed
  const segments: DiffSegment[] = [];
  let i = 0;
  let j = 0;
  const push = (text: string, changed: boolean) => {
    const last = segments[segments.length - 1];
    if (last && last.changed === changed) last.text += text;
    else segments.push({ text, changed });
  };
  while (j < m) {
    if (i < n && a[i] === b[j]) {
      push(b[j], false);
      i++;
      j++;
    } else if (i < n && dp[i + 1][j] >= dp[i][j + 1]) {
      i++; // token removed from original
    } else {
      push(b[j], !/^\s+$/.test(b[j])); // inserted/changed (don't flag pure whitespace)
      j++;
    }
  }
  return segments;
}
