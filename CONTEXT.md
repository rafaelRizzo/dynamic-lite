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
A Island do tamanho exato do Notch, sem conteúdo.

**Compact**:
A Island esticada para os lados do Notch, mostrando um resumo da Activity.
_Avoid_: Minimal, collapsed

**Expanded**:
A Island aberta para baixo com a Activity completa e seus controles.
_Avoid_: Open, full, large

### Conteúdo

**Activity**:
Algo em andamento que a Island pode mostrar (na v1, só Now Playing).
_Avoid_: Widget, module, plugin

**Now Playing**:
A Activity da faixa atual do Spotify: faixa, artista, capa, posição e controles.
_Avoid_: Media, player, music widget
