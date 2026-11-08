import { expect, test } from "bun:test";
import { lineAt, parseLrc } from "./lyrics";

test("lê tempos e textos em ordem", () => {
  const lines = parseLrc("[00:12.50] segunda\n[00:01.00] primeira\n[01:02.5] terceira");
  expect(lines).toEqual([
    { time: 1, text: "primeira" },
    { time: 12.5, text: "segunda" },
    { time: 62.5, text: "terceira" },
  ]);
});

test("ignora metadados e linhas sem tempo, aceita CRLF", () => {
  const lines = parseLrc("[ar:Alguém]\r\n[ti:Algo]\r\nsem tempo\r\n[00:03.000] oi");
  expect(lines).toEqual([{ time: 3, text: "oi" }]);
});

test("vários tempos na mesma linha repetem o texto", () => {
  expect(parseLrc("[00:01.00][00:09.00] refrão")).toEqual([
    { time: 1, text: "refrão" },
    { time: 9, text: "refrão" },
  ]);
});

test("linha vazia mantém o tempo (pausa instrumental)", () => {
  expect(parseLrc("[00:05.00] \n[00:08.00] volta")).toEqual([
    { time: 5, text: "" },
    { time: 8, text: "volta" },
  ]);
});

test("lineAt acha a linha atual", () => {
  const lines = parseLrc("[00:05.00] a\n[00:10.00] b\n[00:20.00] c");
  expect(lineAt(lines, 0)).toBe(-1);
  expect(lineAt(lines, 5)).toBe(0);
  expect(lineAt(lines, 9.99)).toBe(0);
  expect(lineAt(lines, 10)).toBe(1);
  expect(lineAt(lines, 999)).toBe(2);
  expect(lineAt([], 5)).toBe(-1);
});
