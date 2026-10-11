# Gesto de 3 dedos pelo toque bruto do trackpad

Trocar de aba com 3 dedos lê os toques direto do framework privado MultitouchSupport (carregado com `dlopen` em runtime, como o Process Tap do ADR 0005) e detecta 3 dedos andando mais de 10% da largura do trackpad, mais na horizontal que na vertical. O evento de swipe do macOS (`NSEventTypeSwipe`) foi a primeira tentativa e ficou só como reserva: ele só chega ao app quando o gesto de 3 dedos não está reservado pelo sistema (Mission Control, Deslizar entre páginas...), e na prática não disparou. O toque bruto não depende dessas configurações.

- Só conta com exatamente 3 dedos e só age com o mouse sobre a Island (o Rust confere a Hit Region); fora dela o gesto é ignorado.
- As duas fontes podem disparar o mesmo gesto: um intervalo de 450 ms evita trocar duas abas.
- É API privada: pode mudar ou sumir numa versão do macOS. Se o framework não carregar, o app segue sem o gesto (e só o swipe do sistema, se existir, funciona).
- Os gestos do próprio sistema com 3 dedos continuam rodando em paralelo: se algum estiver ligado, vai disparar junto.
