# Janela fixa com click-through controlado por polling do mouse

A janela tem tamanho fixo (o da Island Expanded) e nunca é redimensionada; a forma anima dentro dela. Como o resto da janela é transparente mas ainda bloquearia cliques na menu bar, um thread em Rust lê a posição global do mouse a cada ~16ms (um frame) e liga/desliga `ignoresMouseEvents` conforme o cursor entra/sai da Hit Region. Redimensionar a janela a cada frame da animação causaria tremidas e é caro; eventos de mouse do webview não servem porque, com click-through ligado, ele não recebe nada.
