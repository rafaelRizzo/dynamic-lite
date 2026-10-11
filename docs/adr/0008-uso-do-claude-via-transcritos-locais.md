# Uso do Claude Code: transcritos locais e statusline

Duas fontes locais, sem rede e sem login:

- **Custo e tokens de hoje e o bloco de 5h estimado** vêm de `~/.claude/projects/**/*.jsonl` (`message.usage`). O leitor é incremental (offset por arquivo, só consome linhas completas), deduplica o turno por `message.id` + `requestId` e olha só as últimas 48h, o bastante pra achar o começo do bloco (hora cheia do primeiro uso). O custo é estimativa: tabela de preços por família embutida no app, que pode defasar.
- **Os limites reais do plano** (semanal e de 5h: porcentagem usada e horário de reinício) não estão nos transcritos. Duas fontes:
  1. **Consulta direta à conta** (padrão, desligável nos Ajustes): o app lê o token de acesso do Claude Code no Keychain (`security find-generic-password -s "Claude Code-credentials"`) e pergunta a `api.anthropic.com/api/oauth/usage`, o mesmo que o `/usage` faz. O token fica só em memória, vai pro `curl` pela entrada padrão (nunca em argumentos de processo) e não é salvo nem logado. Não renovamos o token: vencido, quem renova é o Claude Code. Cache de 2 min (5 min depois de falha, pra não insistir num Keychain negado). O endpoint não é documentado e pode mudar.
  2. **Arquivo do statusline** (reserva): o `statusLine` do usuário grava o JSON recebido em `~/.claude/dynano-usage.json` e o app lê. Só atualiza quando o Claude Code roda no terminal: a extensão do VS Code não executa o statusline.

O app só lê com a aba aberta e `showClaudeUsage` ligado. A UI diz se o limite é ao vivo ou de quando foi gravado, e avisa quando o arquivo passa de 30 min.
