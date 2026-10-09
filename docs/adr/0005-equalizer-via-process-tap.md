# Equalizer ao vivo via Core Audio Process Tap

O Equalizer acompanha a música capturando o áudio dos processos do Spotify com um Core Audio Process Tap (`CATapDescription` + aggregate device privado, macOS 14.2+). O Rust passa o áudio por 4 filtros passa-banda (80 Hz, 350 Hz, 1,5 kHz, 6 kHz), mede o quanto cada banda sobe ou desce em relação à própria média recente (batidas, não volume absoluto, que em música comprimida grudaria no topo) e emite os níveis a ~30fps; o frontend escreve direto no DOM, sem re-render.

- A Spotify Web API (`audio-analysis`) foi descartada: fechada pra apps novos desde nov/2024, e exigiria login OAuth.
- ScreenCaptureKit também capturaria o áudio, mas pede Gravação de Tela, permissão bem mais invasiva que a de áudio.
- O tap só existe enquanto o Spotify toca: pausado ou fechado, é destruído e some o indicador de gravação.
- `AudioHardwareCreateProcessTap` é resolvido com `dlsym`: linkar direto quebraria o app no macOS < 14.2 (mínimo é 12).
- Sem suporte, sem permissão (o tap entrega só zeros) ou no silêncio, nada é emitido e o Equalizer volta pra animação CSS.
