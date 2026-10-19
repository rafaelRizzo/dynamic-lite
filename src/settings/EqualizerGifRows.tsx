import { useState } from "react";
import { MAX_GIF_BYTES, native, type Settings } from "../lib/native";
import { Note, Range, Row, Segmented } from "./controls";

const MB = 1024 * 1024;
const formatMb = (bytes: number) => `${(bytes / MB).toFixed(1).replace(".", ",")} MB`;
const LIMIT = `${MAX_GIF_BYTES / MB} MB`;

type Props = { settings: Settings; update: (patch: Partial<Settings>) => void };

/** Estilo do equalizer: barras ou um GIF enviado (até 50 MB), com o tamanho do arquivo e o tamanho na Island. */
export function EqualizerGifRows({ settings, update }: Props) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const hasGif = settings.equalizerGifBytes > 0;

  // o seletor é nativo (só .gif) e o Rust valida tipo e tamanho; aqui só se mostra o resultado
  const pick = async () => {
    setError(null);
    setBusy(true);
    try {
      if (await native.equalizerGifPick()) update({ equalizerStyle: "gif" });
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const remove = () => {
    setError(null);
    native.equalizerGifRemove().catch((e) => setError(String(e)));
  };

  return (
    <>
      <Row label="Estilo do equalizer" hint="Barras ou seu GIF">
        <Segmented
          value={settings.equalizerStyle}
          onChange={(equalizerStyle) => update({ equalizerStyle })}
          options={[
            { value: "bars", label: "Barras" },
            { value: "gif", label: "GIF", disabled: !hasGif, title: hasGif ? undefined : "Envie um GIF primeiro" },
          ]}
        />
      </Row>
      <Row label="GIF" hint={hasGif ? `${formatMb(settings.equalizerGifBytes)} de ${LIMIT}` : `Só .gif, até ${LIMIT}`}>
        <button className="st-button" onClick={pick} disabled={busy}>
          {busy ? "Escolhendo..." : hasGif ? "Trocar" : "Enviar GIF"}
        </button>
        {hasGif && (
          <button className="st-button" onClick={remove} disabled={busy}>
            Remover
          </button>
        )}
      </Row>
      {error && <Note>{error}</Note>}
      {hasGif && settings.equalizerStyle === "gif" && (
        <Row label="Tamanho do GIF" hint="Na Island aberta fica 2x maior">
          <Range
            value={settings.equalizerGifSize}
            min={16}
            max={48}
            step={2}
            format={(v) => `${v}px`}
            onChange={(v) => update({ equalizerGifSize: v })}
          />
        </Row>
      )}
    </>
  );
}
