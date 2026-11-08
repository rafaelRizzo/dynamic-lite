import { expect, test } from "bun:test";
import { baseName, formatSize, presetById, PRESETS } from "./presets";

test("tamanhos legíveis", () => {
  expect(formatSize(512)).toBe("512 B");
  expect(formatSize(1536)).toBe("1,5 KB");
  expect(formatSize(5 * 1024 * 1024)).toBe("5,0 MB");
  expect(formatSize(250 * 1024 * 1024)).toBe("250 MB");
});

test("nome do arquivo em caminhos do Windows e do Mac", () => {
  expect(baseName("C:\\Users\\a\\musica_8k.wav")).toBe("musica_8k.wav");
  expect(baseName("/Users/a/musica.mp3")).toBe("musica.mp3");
});

test("presets têm ids e sufixos únicos", () => {
  expect(new Set(PRESETS.map((p) => p.id)).size).toBe(PRESETS.length);
  expect(new Set(PRESETS.map((p) => p.suffix)).size).toBe(PRESETS.length);
  expect(presetById("gsm")?.options.format).toBe("gsm");
  expect(presetById("nao-existe")).toBeUndefined();
});
