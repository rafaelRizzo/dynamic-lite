# Dynamic Lite

Dynamic Island para macOS: uma superfície preta que vive no notch da tela principal e se transforma para mostrar atividades em andamento.

## Language

### Superfície

**Island**:
A forma preta no topo central da tela principal, encaixada no Notch (ou simulando um).
_Avoid_: Notch app, widget, pill

**Notch**:
O recorte físico da câmera na tela; quando a tela não tem, é o espaço equivalente simulado pela Island.
_Avoid_: Cutout, sensor housing

**Ears**:
Os dois cantos superiores côncavos que fundem a Island com a borda da tela.
_Avoid_: Corners, shoulders

**Hit Region**:
A área da Island que recebe mouse; tudo fora dela é clicável no app que está atrás.
_Avoid_: Hover area, bounds

### Estados

**Idle**:
A Island do tamanho exato do Notch, sem conteúdo; em telas sem Notch pode ficar escondida.

**Compact**:
A Island esticada para os lados do Notch, mostrando um resumo da Activity; preta, exceto no Style Themed, que usa a cor da capa.
_Avoid_: Minimal, collapsed

**Expanded**:
A Island aberta para baixo com o Panel ativo e, embaixo, a barra de abas.
_Avoid_: Open, full, large

### Aparência

**Style**:
A aparência do Expanded: Black, Translucent, Glass ou Themed (fundo na cor da capa da música).
_Avoid_: Theme, skin, mode

**Target Screen**:
A tela onde a Island vive: Automática (a com Notch, senão a principal), Principal ou uma tela escolhida pelo nome.
_Avoid_: Monitor, display

### Ajustes

**Settings**:
As preferências do usuário, aplicadas na hora e persistidas entre execuções.
_Avoid_: Config, preferences, options

**Context Menu**:
O menu nativo aberto pelo clique direito na Island, com acesso à Settings Window.
_Avoid_: Right-click menu, popup

**Settings Window**:
A janela própria onde as Settings são editadas.
_Avoid_: Preferences pane, config screen

### Conteúdo

**Activity**:
Algo em andamento que aparece no Compact (música tocando, Pomodoro rodando, atualização disponível). Existir já significa estar viva.
_Avoid_: Widget, module, plugin

**Slot**:
Um dos dois lados do Notch no Compact. Cada Slot mostra a primeira Activity, por prioridade, que o preenche.
_Avoid_: Side, column

**Panel**:
Uma aba do Expanded: Música, Agenda, Pomodoro, Claude Code, Atalhos, Atualização. Todas as abas têm o tamanho da maior, pra barra de abas não pular ao trocar.
_Avoid_: Tab content, page, screen

**Alert**:
Aviso que abre a Island sozinha por alguns segundos, com som: fim de fase do Pomodoro, Lembrete ou evento prestes a começar. Fica aberto enquanto o mouse estiver nela.
_Avoid_: Notification, toast

**Pomodoro**:
Ciclo de Foco, Pausa curta e Pausa longa (a cada N focos), cada fase de 1 minuto a 24 horas. O tempo mora no Rust.
_Avoid_: Timer, countdown

**Reminder**:
Texto com hora que o app guarda e avisa na hora marcada. Concluir apaga.
_Avoid_: Task, todo, event

**Event**:
Compromisso lido do app Calendário do macOS; o Dynamic Lite só lê.
_Avoid_: Appointment, meeting

**Claude Usage**:
Custo e tokens do Claude Code hoje e na Sessão de 5h em andamento, estimados dos transcritos locais.
_Avoid_: Billing, quota

**Shortcut**:
Botão que abre um painel dos Ajustes do Sistema; o usuário escolhe quais aparecem.
_Avoid_: Quick action, link

**Now Playing**:
A Activity da faixa atual do Spotify: faixa, artista, capa, posição e controles.
_Avoid_: Media, player, music widget

**Equalizer**:
As 4 barras do Now Playing que seguem o áudio do Spotify ao vivo (grave → agudo); sem permissão de áudio, viram animação em loop.
_Avoid_: Visualizer, spectrum, waveform
