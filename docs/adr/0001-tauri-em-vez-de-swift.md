# Tauri em vez de Swift/SwiftUI nativo

A Island é uma janela Tauri transparente (webview + React) e só o que o webview não alcança (nível da janela, geometria do Notch, mouse global) é feito em Rust via objc2. Swift/SwiftUI daria animação mais fiel e ~50MB a menos de RAM, mas o projeto já nasceu em Tauri e a UI em React é onde o time é produtivo; a animação "líquida" com molas em Motion ficou boa o bastante.
