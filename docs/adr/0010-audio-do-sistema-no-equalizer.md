# Áudio do sistema no Equalizer, sem metadados

Com a opção "Seguir o áudio do sistema" ligada e o Spotify parado, o Equalizer passa a capturar a mixagem de todos os apps com um Core Audio Process Tap global (`initStereoGlobalTapButExcludeProcesses`) e a Island mostra uma Activity (capa ou nota à esquerda, Equalizer à direita) e uma aba "Áudio". O Spotify tocando continua mandando (tap só nos processos dele, Panel Música).

- Sem MediaRemote (bloqueado para apps de terceiros desde o macOS 15.4, ADR 0002), o título vem da aba do navegador por AppleScript (`system_media.rs`): o app com saída de áudio aberta (`com.google.Chrome.helper` → Chrome) é consultado, e a aba ativa ou, se ela não for de mídia, a primeira de um site conhecido (YouTube, SoundCloud...) vira o título. A capa só existe no YouTube (`i.ytimg.com/vi/<id>`, a partir da URL). Custa um prompt de Automação por navegador; negado, fica só a nota e o nome do app.
- O AppleScript não diz qual aba é audível: com vários vídeos abertos fora da aba ativa, pode mostrar o errado.
- Outros players (Música, VLC) mostram só o nome do app.
- O tap só existe enquanto algum processo (fora o próprio app) tem saída de áudio aberta (`kAudioProcessPropertyIsRunningOutput`, sem permissão, olhada a cada 1 s). Apps que mantêm a saída aberta em silêncio (chamadas) deixam o indicador de gravação ligado, mas a Activity só aparece com som de verdade (2 s de silêncio a escondem).
- Desligada por padrão: ao contrário do Spotify, não dá pra saber de antemão que o som é música, então fica a critério do usuário.
- A mesma permissão de áudio do Equalizer do Spotify vale; negada, o tap entrega zeros e a Activity nunca aparece.
