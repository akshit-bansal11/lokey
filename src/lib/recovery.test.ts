import assert from "node:assert/strict";
import { test } from "node:test";
import { sameRecoveryKey } from "./recovery.ts";

const SHOWN = "0A1B-2C3D-4E5F-6G7H-8J9K-MNPQ-RSTV-WXYZ";

test("sameRecoveryKey accepts the key as shown", () => {
  assert.equal(sameRecoveryKey(SHOWN, SHOWN), true);
});

test("sameRecoveryKey ignores case, dashes and spaces", () => {
  assert.equal(sameRecoveryKey("0a1b 2c3d4e5f 6g7h8j9kmnpqrstvwxyz", SHOWN), true);
});

test("sameRecoveryKey reads O as 0 and I or L as 1", () => {
  assert.equal(sameRecoveryKey("OA1B-2C3D-4E5F-6G7H-8J9K-MNPQ-RSTV-WXYZ", SHOWN), true);
  assert.equal(sameRecoveryKey("0AIB-2C3D-4E5F-6G7H-8J9K-MNPQ-RSTV-WXYZ", SHOWN), true);
});

test("sameRecoveryKey rejects a key with one character wrong", () => {
  assert.equal(sameRecoveryKey("0A1B-2C3D-4E5F-6G7H-8J9K-MNPQ-RSTV-WXY0", SHOWN), false);
});

test("sameRecoveryKey rejects an empty answer", () => {
  assert.equal(sameRecoveryKey("", ""), false);
});
