import assert from "node:assert/strict";
import { test } from "node:test";
import { nameProblem, projectNames, visibleRows, wouldReplace } from "./rows.ts";
import type { Row } from "./types.ts";

function row(overrides: Partial<Row>): Row {
  return { project: "default", key: "KEY", length: 3, updated: 0, ...overrides };
}

test("visibleRows keeps only the chosen project", () => {
  const rows = [row({ key: "A" }), row({ key: "B", project: "web" })];

  const shown = visibleRows(rows, "web", "");

  assert.deepEqual(
    shown.map((r) => r.key),
    ["B"],
  );
});

test("visibleRows matches project case-insensitively", () => {
  const rows = [row({ key: "A", project: "Web" })];

  const shown = visibleRows(rows, "web", "");

  assert.equal(shown.length, 1);
});

test("visibleRows filters by search text ignoring case", () => {
  const rows = [row({ key: "DATABASE_URL" }), row({ key: "API_KEY" })];

  const shown = visibleRows(rows, "default", "data");

  assert.deepEqual(
    shown.map((r) => r.key),
    ["DATABASE_URL"],
  );
});

test("visibleRows sorts keys alphabetically", () => {
  const rows = [row({ key: "b" }), row({ key: "A" }), row({ key: "C" })];

  const shown = visibleRows(rows, "default", "");

  assert.deepEqual(
    shown.map((r) => r.key),
    ["A", "b", "C"],
  );
});

test("projectNames puts default first then sorts the rest", () => {
  const names = projectNames(
    [
      { name: "web", count: 1 },
      { name: "api", count: 2 },
    ],
    [],
  );

  assert.deepEqual(names, ["default", "api", "web"]);
});

test("projectNames adds a pending project once", () => {
  const names = projectNames([{ name: "web", count: 1 }], ["WEB", "mobile"]);

  assert.deepEqual(names, ["default", "mobile", "web"]);
});

test("wouldReplace is true for an existing key in any case", () => {
  const rows = [row({ key: "API_KEY", project: "web" })];

  assert.equal(wouldReplace(rows, "web", "api_key"), true);
});

test("wouldReplace is false for the same key in another project", () => {
  const rows = [row({ key: "API_KEY", project: "web" })];

  assert.equal(wouldReplace(rows, "api", "API_KEY"), false);
});

test("nameProblem rejects an equals sign", () => {
  assert.match(nameProblem("key", "A=B"), /cannot contain "="/);
});

test("nameProblem accepts env style names", () => {
  assert.equal(nameProblem("key", "NEXT_PUBLIC_URL.v2/x-y"), "");
});
