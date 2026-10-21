# Arquitectura

Wispr Swag es un único proceso nativo de Windows escrito en Rust. La interfaz de
ajustes usa Tauri 2 (WebView2), pero solo existe mientras la ventana está
abierta; todo lo que interviene en un dictado es nativo.

```
┌─────────────────────────── wispr-swag.exe ───────────────────────────┐
│                                                                      │
│  Hilo del gancho ──eventos──▶ Controlador ──órdenes──▶ Barra flotante│
│  (WH_KEYBOARD_LL)            (tokio, estado del         (ventana por │
│                               dictado y cola)            capas, hilo │
│                                   │                      propio)     │
│                                   ▼                                  │
│                    Captura de audio (cpal/WASAPI, solo al grabar)    │
│                                   │                                  │
│                                   ▼                                  │
│          Procesador: DSP → VAD → motor de voz → texto → (IA) →       │
│          inserción (SendInput / portapapeles) → historial            │
│                                                                      │
│  Bandeja del sistema · Mantenimiento cada 6 h · Ventana de ajustes   │
│  (WebView2, bajo demanda, se destruye al cerrarla)                   │
└──────────────────────────────────────────────────────────────────────┘
```

## Componentes

### `crates/wispr-core` (lógica sin interfaz)

| Módulo | Responsabilidad |
|---|---|
| `audio::capture` | Grabación con cpal (WASAPI) en un hilo dedicado; nivel en vivo, detección de desconexión y vuelta al micrófono predeterminado. |
| `audio::dsp` | Mezcla a mono, remuestreo sinc con ventana de Kaiser a 16 kHz, filtro paso alto Butterworth a 80 Hz, normalización por pico robusto. |
| `audio::vad` | Detección de voz por energía con suelo de ruido adaptativo; recorte de silencios y segmentación por pausas para la transcripción incremental. |
| `stt::local` | Motor sherpa-onnx con caché de un solo modelo, carga perezosa, liberación por inactividad y devolución de la memoria al sistema. |
| `stt::catalog`, `stt::download` | Catálogo de modelos con tamaños y SHA-256; descarga reanudable con extracción selectiva y verificación. |
| `stt::chunk` | División de audios largos por el punto más silencioso (Whisper admite 30 s por pasada). |
| `stt::cloud` | Transcripción por `/audio/transcriptions` (OpenAI, Groq o servidor propio) con el vocabulario como pista. |
| `text::*` | Puntuación dictada, limpieza (muletillas, repeticiones, arranques en falso), filtro de frases inventadas, vocabulario con corrección fonética, fragmentos, comandos de voz, modos y formato (párrafos, viñetas, correo, código). |
| `llm::*` | Proveedores de IA (API de Anthropic y API compatible con OpenAI), indicaciones con protección frente a inyección, validación de la respuesta y control de uso y coste. |
| `settings`, `history`, `secrets`, `appctx`, `hotkey`, `sysinfo` | Ajustes con copias de seguridad, historial SQLite con estadísticas, credenciales en el Administrador de credenciales, perfiles por aplicación, modelo de atajos y medición del proceso. |

### `crates/wispr-pill`

Dibuja la barra con tiny-skia y fontdue (tipografía Inter incrustada) en un
búfer RGBA premultiplicado. No sabe nada de ventanas: el host de Windows copia
el búfer a una DIB y lo muestra con `UpdateLayeredWindow`. Incluye la
animación (suavizado exponencial independiente de la tasa de fotogramas), la
forma de onda y el cálculo de zonas de clic.

### `src-tauri`

| Módulo | Responsabilidad |
|---|---|
| `controller` | Máquina de estados del dictado: modos híbrido/mantener/pulsar, transcripción incremental mientras se habla, vigilante de duración y desconexión, cola de trabajos cancelable, reintento y comandos de voz. |
| `hotkey_engine` | Máquina de estados de los atajos, independiente de Windows y con pruebas: acordes de modificadores, anulación al pulsar otra tecla, crecimiento a modo comando, captura de atajos nuevos y enmascarado de Win/Alt/Mayús. |
| `platform::windows` | Gancho `WH_KEYBOARD_LL` en su hilo, barra como ventana `WS_EX_LAYERED | WS_EX_NOACTIVATE | WS_EX_TOPMOST`, inserción con `SendInput` (pegado o Unicode), portapapeles con copia y restauración excluidas del historial de Windows, ventana activa (incluidas apps UWP), elevación, pantalla completa, inicio con Windows, sonidos sintetizados y silencio del audio del sistema. |
| `processor` | Une motor de voz, reglas de texto, perfil de la aplicación e IA. |
| `commands`, `hub`, `tray` | API para la interfaz, ventana de ajustes bajo demanda y menú de la bandeja. |
| `selftest` | Autoprueba del ejecutable final en Windows. |

### `src` (interfaz)

Svelte 5 con runas, TypeScript estricto y un sistema de diseño propio
(tokens monocromo en `styles/tokens.css`, temas claro/oscuro/sistema). Los
ajustes se guardan solos (con 280 ms de espera) y el backend los valida. En un
navegador normal usa un backend simulado (`lib/mock.ts`) para trabajar el
diseño sin Windows.

## Flujo de un dictado

1. **Atajo.** El gancho pasa cada pulsación a `hotkey_engine` y bloquea solo lo
   necesario. Al soltar Win/Alt inyecta una tecla neutra (`vkE8`) para que no se
   abra Inicio ni el menú.
2. **Grabación.** Se recuerda la ventana activa, se abre el micrófono y la barra
   muestra la forma de onda. El modelo local empieza a cargarse en paralelo si no
   estaba en memoria.
3. **Transcripción incremental.** Con el motor local, cada frase se transcribe
   al detectar una pausa, mientras el usuario sigue hablando.
4. **Final.** Al soltar, se transcribe solo lo que falta. El texto pasa por las
   reglas locales del modo (y por la IA si está activada; si falla o tarda, se
   usa el resultado local).
5. **Inserción.** Pegado con restauración del portapapeles (o escritura Unicode
   según el perfil). Si no hay campo de texto, o la aplicación se ejecuta como
   administrador, el texto queda en el portapapeles y la barra lo indica.
6. **Registro.** Historial y estadísticas (configurable); el audio solo se guarda
   si se ha activado.

## Decisiones de rendimiento

- Sin WebView en reposo: la barra es nativa y la ventana de ajustes se destruye
  al cerrarla (por defecto).
- La barra solo programa un temporizador mientras anima; en reposo, cero CPU.
- El modelo se carga al empezar a hablar (en paralelo), se libera tras 10 min sin
  uso (configurable) y la memoria se devuelve al sistema.
- El tipo de modelo se indica a sherpa-onnx para evitar que lo cargue dos veces.
- Remuestreo, filtrado y VAD propios, sin dependencias pesadas.
- Compilación con LTO, `strip` y CRT estático.
