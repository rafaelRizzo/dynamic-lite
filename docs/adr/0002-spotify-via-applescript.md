# Now Playing só do Spotify, via AppleScript

Lemos e controlamos o Spotify por AppleScript (`osascript`), disparado pela notificação distribuída `com.spotify.client.PlaybackStateChanged` mais um polling lento de segurança. O MediaRemote (framework privado) cobriria qualquer player, mas desde o macOS 15.4 bloqueia apps de terceiros e exige gambiarras frágeis. Custo: só Spotify e um prompt de permissão de Automação na primeira vez.
