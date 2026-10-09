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
A Island aberta para baixo com a Activity completa e seus controles.
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
Algo em andamento que a Island pode mostrar (na v1, só Now Playing).
_Avoid_: Widget, module, plugin

**Now Playing**:
A Activity da faixa atual do Spotify: faixa, artista, capa, posição e controles.
_Avoid_: Media, player, music widget
