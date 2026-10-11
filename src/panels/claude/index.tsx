import { useEffect, useState } from "react";
import type { Panel } from "../../activities/types";
import { useNow } from "../../hooks/useNow";
import { native, type ClaudeUsage, type Settings, type UsageLimit, type UsageTotals } from "../../lib/native";
import { ClaudeIcon } from "../icons";

const WIDTH = 400;
const HEIGHT = 150;
const REFRESH_MS = 30_000;
/** Sem o Claude Code aberto o arquivo não atualiza: depois disso o dado é tratado como velho. */
const STALE_MS = 30 * 60_000;

const usd = (v: number) => `$${v.toFixed(v >= 100 ? 0 : 2)}`;

function tokens(n: number): string {
  if (n >= 1e6) return `${(n / 1e6).toFixed(1)}M`;
  if (n >= 1e3) return `${Math.round(n / 1e3)}k`;
  return String(n);
}

/** "2d 3h", "4h05", "12 min". */
function left(ms: number): string {
  const m = Math.max(0, Math.ceil(ms / 60_000));
  if (m >= 24 * 60) return `${Math.floor(m / 1440)}d ${Math.floor((m % 1440) / 60)}h`;
  return m >= 60 ? `${Math.floor(m / 60)}h${String(m % 60).padStart(2, "0")}` : `${m} min`;
}

const ago = (ms: number) => (ms < 60_000 ? "agora" : `há ${left(ms)}`);

/** Cor da barra por quanto do limite já foi usado. */
const levelOf = (used: number) => (used >= 90 ? "high" : used >= 70 ? "mid" : "low");

function Bar({ value, level }: { value: number; level?: string }) {
  return (
    <div className="cc-bar" data-level={level}>
      <span style={{ width: `${Math.min(100, Math.max(0, value))}%` }} />
    </div>
  );
}

type StatProps = { title: string; big: string; sub?: string; children?: React.ReactNode; subTitle?: string };

function Stat({ title, big, sub, subTitle, children }: StatProps) {
  return (
    <div className="cc-stat">
      <div className="cc-title">{title}</div>
      <div className="cc-cost">{big}</div>
      {children}
      {sub && <div className="cc-sub" data-tip={subTitle}>{sub}</div>}
    </div>
  );
}

const totalsSub = (t: UsageTotals) => `${tokens(t.input + t.output)} tokens`;

/** Limite real do plano (vem do statusline): porcentagem usada e quando reinicia. */
function LimitStat({ title, limit, now, stale }: { title: string; limit: UsageLimit; now: number; stale: boolean }) {
  const over = limit.resetsAt <= now;
  const day = new Date(limit.resetsAt).toLocaleDateString("pt-BR", { weekday: "short" }).replace(".", "");
  const hour = new Date(limit.resetsAt).toLocaleTimeString("pt-BR", { hour: "2-digit", minute: "2-digit" });
  return (
    <Stat
      title={title}
      big={over ? "0%" : `${Math.round(limit.used)}%`}
      sub={over ? "reiniciou" : `reinicia em ${left(limit.resetsAt - now)}`}
      subTitle={`Reinicia ${day} ${hour}`}
    >
      <Bar value={over ? 0 : limit.used} level={stale ? "stale" : levelOf(limit.used)} />
    </Stat>
  );
}

const LIVE_ERROR: Record<string, string> = {
  keychain: "não consegui ler o token do Claude Code no Keychain",
  expired: "o token do Claude Code venceu (abra o Claude Code uma vez pra renovar)",
  network: "a consulta à conta falhou",
};

/** Explica de onde vem o limite e, se a consulta ao vivo falhou, por quê. */
function footTitle(usage: ClaudeUsage): string {
  if (usage.limits?.live) return "Consultado agora na conta, como o /usage do Claude Code.";
  const why = usage.liveError ? ` Consulta ao vivo falhou: ${LIVE_ERROR[usage.liveError]}.` : "";
  return `Vem do statusline do Claude Code, que só grava quando você usa o Claude Code no terminal.${why}`;
}

/** Último resultado: ao voltar pra aba mostra na hora e atualiza por baixo, em vez de esperar a leitura. */
let lastUsage: ClaudeUsage | null = null;

function Expanded({ topInset }: { topInset: number }) {
  const [usage, setUsage] = useState<ClaudeUsage | null>(lastUsage);
  const [failed, setFailed] = useState(false);
  const now = useNow(30_000);

  useEffect(() => {
    const load = () =>
      native
        .claudeUsage(new Date(new Date().setHours(0, 0, 0, 0)).getTime())
        .then((u) => {
          lastUsage = u;
          setUsage(u);
          setFailed(false);
        })
        .catch((e) => {
          console.error("claude_usage falhou:", e);
          setFailed(true);
        });
    load();
    const id = setInterval(load, REFRESH_MS);
    return () => clearInterval(id);
  }, []);

  const limits = usage?.limits;
  const stale = !!limits && !limits.live && now - limits.updatedAt > STALE_MS;
  const block = usage?.block;
  const elapsed = block ? Math.min(100, Math.max(0, ((now - block.start) / (block.end - block.start)) * 100)) : 0;

  return (
    <div className="cc-expanded" style={{ paddingTop: topInset }}>
      {!usage ? (
        <div className="cc-empty">{failed ? "Não consegui ler o uso do Claude Code" : "Lendo o uso…"}</div>
      ) : (
        <>
          <div className="cc-cols">
            {limits?.sevenDay ? (
              <LimitStat title="Semana" limit={limits.sevenDay} now={now} stale={stale} />
            ) : (
              <Stat title="Semana" big="--" sub="statusline sem dados" subTitle="Veja como ligar em Ajustes > Claude Code" />
            )}
            {limits?.fiveHour ? (
              <LimitStat title="Sessão de 5h" limit={limits.fiveHour} now={now} stale={stale} />
            ) : block ? (
              <Stat title="Sessão de 5h" big={usd(block.totals.cost)} sub={`reinicia em ${left(block.end - now)}`} subTitle="Estimado pelos transcritos locais">
                <Bar value={elapsed} />
              </Stat>
            ) : (
              <Stat title="Sessão de 5h" big="--" sub="sem uso nas últimas 5h" />
            )}
            <Stat title="Hoje" big={usd(usage.today.cost)} sub={totalsSub(usage.today)} subTitle={`${usage.today.messages} mensagens. Custo estimado, tokens de entrada + saída sem cache`}>
              <div className="cc-bar-gap" />
            </Stat>
          </div>
          <div className="cc-foot" data-stale={stale} data-tip={footTitle(usage)}>
            {limits
              ? limits.live
                ? "Limite em tempo real"
                : `Limite atualizado ${ago(now - limits.updatedAt)}`
              : "Valores estimados pelos transcritos locais"}
          </div>
        </>
      )}
    </div>
  );
}

/** Uso do Claude Code: limite semanal e de 5h do plano (via statusline) e custo de hoje; só lê com a aba aberta. */
export function useClaudePanel(settings: Settings): Panel | null {
  if (!settings.showClaudeUsage) return null;
  return {
    id: "claude",
    label: "Claude Code",
    icon: <ClaudeIcon />,
    expanded: ({ topInset }) => <Expanded topInset={topInset} />,
    expandedSize: { width: WIDTH, height: HEIGHT },
  };
}
