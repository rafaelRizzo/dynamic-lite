import { expect, test } from "bun:test";
import { parseReminder } from "./parse";

const now = new Date(2026, 9, 11, 18, 0);
const at = (d: number, h: number, m = 0) => new Date(2026, 9, d, h, m).getTime();

test("minutos relativos", () => {
  expect(parseReminder("20m ligar pro João", now)).toEqual({ at: now.getTime() + 20 * 60_000, text: "ligar pro João" });
  expect(parseReminder("em 5 min café", now)?.text).toBe("café");
});

test("horas só com 'em'/'daqui'", () => {
  expect(parseReminder("em 2h revisar", now)?.at).toBe(now.getTime() + 120 * 60_000);
  // "9h" sozinho é hora do relógio, não 9 horas a partir de agora
  expect(parseReminder("9h dentista", now)?.at).toBe(at(12, 9));
});

test("hora do relógio: hoje se ainda não passou, senão amanhã", () => {
  expect(parseReminder("19:30 reunião", now)?.at).toBe(at(11, 19, 30));
  expect(parseReminder("17:00 reunião", now)?.at).toBe(at(12, 17));
  expect(parseReminder("15h30 reunião", now)?.at).toBe(at(12, 15, 30));
});

test("amanhã e dia selecionado", () => {
  expect(parseReminder("amanhã 9h pagar", now)?.at).toBe(at(12, 9));
  expect(parseReminder("às 8:15 treino", now, new Date(2026, 9, 14))?.at).toBe(at(14, 8, 15));
});

test("entradas inválidas", () => {
  expect(parseReminder("25:00 x", now)).toBeNull();
  expect(parseReminder("20m", now)).toBeNull();
  expect(parseReminder("comprar pão", now)).toBeNull();
});
