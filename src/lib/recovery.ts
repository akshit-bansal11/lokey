// Recovery keys as people copy them: case, dashes and spaces do not matter,
// and O, I and L read as 0, 1 and 1, the same way lokey-core reads them.

export function normalizeRecoveryKey(text: string): string {
  return text.toUpperCase().replace(/[\s-]/g, "").replace(/O/g, "0").replace(/[IL]/g, "1");
}

/** Whether what was typed back is the key that was shown. */
export function sameRecoveryKey(typed: string, shown: string): boolean {
  const normal = normalizeRecoveryKey(typed);
  return normal !== "" && normal === normalizeRecoveryKey(shown);
}
