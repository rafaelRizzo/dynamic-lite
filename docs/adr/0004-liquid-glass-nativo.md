# Liquid Glass nativo atrás do webview, recortado no formato da Island

O Style Glass usa um `NSGlassEffectView` (macOS 26+) cobrindo a janela inteira, atrás do webview transparente, recortado por uma máscara (`CAShapeLayer`) com o contorno da Island, Ears incluídas. O frontend manda a forma alvo; o Rust anima o contorno da máscara com `CASpringAnimation` usando a mesma física de mola do React (stiffness 560, damping 31, mass 0.9), então vidro e conteúdo andam juntos. O React desenha só o conteúdo por cima.

- `backdrop-filter` em CSS foi descartado: o webview não enxerga o que está atrás da janela, então não haveria refração real.
- O vidro nativo só sabe ser retângulo arredondado; a máscara é o que permite as Ears côncavas e o vidro grudado no topo. Custo: a borda especular do vidro fica de fora do recorte.
- Idle e Compact continuam pretos pra fundir com o Notch; o vidro nasce da forma do Compact/Idle e volta pra ela.
- Em macOS sem `NSGlassEffectView`, o Style Glass cai para Translucent.
