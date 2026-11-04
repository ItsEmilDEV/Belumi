# Belumi Voice Chat

Belumi can talk to servers that run the [Simple Voice Chat](https://modrinth.com/plugin/simple-voice-chat) mod or plugin
(compatibility version 21, the 2.6.x line). The client side is the crate `crates/belumi-voice`; the game glue is
`vendor/horizon/crates/horizon-client/src/game/belumi_voice.rs`.

## Status

Verified: the wire format (secret packet, UDP framing, AES-128-GCM, all packet kinds), Opus encoding and decoding, the
connect sequence, microphone gating, positional mixing and the timeout, against a fake server that implements the
protocol as the server mod documents it in its source (28 tests, `cargo test -p belumi-voice`).

Not verified: a live Simple Voice Chat server and real audio hardware. Treat the first session on a real server as a
test. If a server refuses the client, `Voice chat lost: ...` or silence in chat is the symptom; please report the
server's mod version.

## Turn it on

1. Build the client with `--features belumi-voice` (in `vendor/horizon`). It needs `libopus` (`pacman -S opus`,
   `apt install libopus-dev`) and the ALSA headers on Linux.
2. Options > Horizon Settings > Belumi Settings > Voice chat > **Voice chat: On**.
3. Join a server with the voice mod. The client sends `voicechat:request_secret`; the server answers with
   `voicechat:secret`, and the client connects to the server's UDP voice port. The chat says `Voice chat connected.`

A build without the feature says so in chat once when the switch is on.

## Controls and settings

| Setting | Default | Notes |
|---|---|---|
| Microphone mode | Push to talk | or voice activation |
| Push-to-talk key | V | held to transmit |
| Mute key | K | toggles muting |
| Activation level | -50 dB | voice activation mode only |
| Microphone volume | 100% | 50% to 200% |
| Voice volume | 100% | 50% to 200% |

All of them live in `belumi.toml`. The HUD shows `Mic on`, `Mic ready` or `Mic muted` and who is speaking, in the HUD
corner.

## How sound is placed

Player voices fade out linearly to the server's configured distance and pan left or right by where the speaker stands
relative to where you look (equal power, so loudness stays constant). Group voices play centered at full volume.
Locational sounds (a server plugin's sound sources) use their own position.

## Protocol summary

- Play channel `voicechat:request_secret` (serverbound): int compatibility version.
- Play channel `voicechat:secret` (clientbound): 16 byte secret, connection UUID, UDP port, player UUID, codec byte,
  MTU, distance, keep-alive, groups flag, voice host, recording flag.
- UDP datagram, client to server: `0xFF`, connection UUID, VarInt length, `IV(12) || AES-128-GCM(payload)`; server to client
  the same without the UUID. Payload: packet type byte and fields. Types: 1 mic, 2 player sound, 3 group sound,
  4 location sound, 5 authenticate, 6 authenticate ack (nonce), 7 ping, 8 keep-alive, 9 connection check (nonce),
  10 connection check ack.
- Audio: Opus, 48 kHz, mono, 20 ms frames (960 samples). An empty Opus payload marks the end of speech.
- The client sends `authenticate` every second until acknowledged, then `connection check` until acknowledged, answers pings
  and keep-alives, and gives up when the server is silent for ten keep-alive intervals.

## Limits

Group management screens, per-player volume sliders, whispering and recording are not part of this module. The client
never records audio.
