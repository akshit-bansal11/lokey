// Pure view logic over a snapshot: no DOM, no I/O, so it is unit-tested.

import type { Project, Row } from "./types.ts";

export const DEFAULT_PROJECT = "default";

/** Names match case-insensitively, as they do in lokey-core. */
export function sameName(a: string, b: string): boolean {
  return a.toLowerCase() === b.toLowerCase();
}

/** The rows of one project matching the search text, sorted by key. */
export function visibleRows(rows: Row[], project: string, query: string): Row[] {
  const needle = query.trim().toLowerCase();
  return rows
    .filter((row) => sameName(row.project, project))
    .filter((row) => needle === "" || row.key.toLowerCase().includes(needle))
    .sort((a, b) => a.key.localeCompare(b.key, undefined, { sensitivity: "base" }));
}

/**
 * Project names for the picker: saved projects, plus any the user created
 * this session that have no key yet, with `default` always present and first.
 */
export function projectNames(saved: Project[], pending: string[]): string[] {
  const names = [DEFAULT_PROJECT];
  for (const name of [...saved.map((p) => p.name), ...pending]) {
    if (!names.some((known) => sameName(known, name))) names.push(name);
  }
  const [first, ...rest] = names;
  return [first ?? DEFAULT_PROJECT, ...rest.sort((a, b) => a.localeCompare(b))];
}

/** Whether saving `key` in `project` would replace an existing value. */
export function wouldReplace(rows: Row[], project: string, key: string): boolean {
  return rows.some((row) => sameName(row.project, project) && sameName(row.key, key.trim()));
}

/** The name rule from lokey-core, checked here only to give instant feedback. */
export function nameProblem(kind: string, name: string): string {
  if (name === "") return `Enter a ${kind} name.`;
  if ([...name].length > 128) return `The ${kind} name is longer than 128 characters.`;
  const bad = [...name].find((c) => !/[\p{L}\p{N}_\-./]/u.test(c));
  return bad === undefined
    ? ""
    : `The ${kind} name cannot contain "${bad}". Use letters, digits and _ - . /`;
}
