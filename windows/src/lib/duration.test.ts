import { expect, test } from "bun:test";
import { formatDuration, parseDuration } from "./duration";

test("aceita os formatos comuns", () => {
  expect(parseDuration("25")).toBe(25);
  expect(parseDuration("25 min")).toBe(25);
  expect(parseDuration("1h")).toBe(60);
  expect(parseDuration("1h30")).toBe(90);
  expect(parseDuration("1:30")).toBe(90);
  expect(parseDuration("24h")).toBe(1440);
});

test("limites: 1 minuto a 24 horas", () => {
  expect(parseDuration("0")).toBeNull();
  expect(parseDuration("1")).toBe(1);
  expect(parseDuration("1440")).toBe(1440);
  expect(parseDuration("1441")).toBeNull();
  expect(parseDuration("24h01")).toBeNull();
  expect(parseDuration("abc")).toBeNull();
});

test("formata", () => {
  expect(formatDuration(25)).toBe("25 min");
  expect(formatDuration(90)).toBe("1h30");
  expect(formatDuration(120)).toBe("2h");
  expect(formatDuration(65)).toBe("1h05");
});
