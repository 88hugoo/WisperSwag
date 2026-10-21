# Wispr Swag

Dictado por voz para todo Windows. Pulsas un atajo, hablas y el texto aparece
—limpio, puntuado y con el tono adecuado— donde esté el cursor: correo, chats,
documentos, código o el navegador.

- **Local y privado por defecto.** El reconocimiento de voz se hace en tu equipo
  con [sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx) y el modelo
  NVIDIA Parakeet V3 (español de España, inglés y 23 idiomas europeos más,
  con detección automática). Sin cuenta, sin servidores, sin telemetría.
- **Ligero.** Un solo proceso nativo: icono de la bandeja, gancho de teclado y
  una barra flotante dibujada por la propia aplicación. La ventana de ajustes
  (WebView2) solo existe mientras está abierta y el modelo de voz se libera de
  la memoria tras unos minutos sin dictar.
- **Barra flotante** compacta, que nunca roba el foco, con estados claros
  (escuchando, procesando, hecho, aviso, error con «Reintentar»).
- **Texto cuidado.** Limpieza de muletillas, repeticiones y arranques en falso
  sin tocar el contenido, puntuación dictada («punto y aparte», «nueva línea»…),
  12 modos de escritura (Natural, Profesional, Conciso, Formal, Informal,
  Correo, Mensajes, Programación, Notas, Lista de tareas, Creativo y
  Personalizado), vocabulario propio con corrección por sonido, fragmentos de
  texto y comandos de voz.
- **IA opcional** (OpenAI, Anthropic, Gemini, Groq, Mistral, OpenRouter, Ollama,
  LM Studio o cualquier servidor compatible) con consentimiento explícito,
  límites de uso y de coste, y vuelta automática al texto local si falla.

> Proyecto independiente, sin relación con Wispr Flow.

## Atajos por defecto

| Atajo | Acción |
|---|---|
| **Ctrl + Win** | Dictar. Mantén pulsado mientras hablas y suelta para insertar; púlsalo una vez (toque corto) para dictar sin manos y otra vez para terminar. |
| **Ctrl + Win + Alt** | Modo comando (IA): instrucción sobre el texto seleccionado o para generar uno nuevo. Basta con añadir Alt mientras dictas. |
| **Esc** | Cancelar. Solo se intercepta mientras grabas o procesas. |
| **Alt + Mayús + Z** | Pegar de nuevo el último dictado. |
| **Alt + Mayús + R** | Reescribir el texto seleccionado con el modo activo (IA). |
| **Alt + Mayús + B** | Mostrar u ocultar la barra en reposo. |

Todos se cambian en **Ajustes › Dictado y atajos**, con aviso de conflictos con
atajos de Windows y de aplicaciones conocidas. Ctrl + Win no interfiere con
Ctrl + Win + ←/→/D (escritorios virtuales): si pulsas otra tecla enseguida, el
dictado se anula y la combinación llega a Windows. Al soltar Win no se abre el
menú Inicio, y Alt + Mayús + tecla no cambia la distribución del teclado.

## Instalación en Windows

Requisitos: Windows 10 (1809 o posterior) u 11 de 64 bits. WebView2 viene con
Windows 11 y con Windows 10 actualizado; si falta, el instalador lo descarga.

1. Descarga `Wispr Swag_x.y.z_x64-setup.exe` (o el `.msi`) de la última
   ejecución correcta de **Actions › CI** (artefacto `wispr-swag-windows`) o de
   *Releases* cuando haya versiones publicadas.
2. Ejecútalo. Se instala para tu usuario, sin permisos de administrador, en
   `%LOCALAPPDATA%\Wispr Swag`.
3. Al abrirse por primera vez, el asistente te guía: micrófono, descarga del
   modelo de voz (≈465 MB, una sola vez, verificado con SHA-256), atajo y una
   prueba real de dictado.

El instalador no está firmado digitalmente: Windows SmartScreen puede pedir
confirmación («Más información › Ejecutar de todas formas»).

Para desinstalar: *Configuración › Aplicaciones › Wispr Swag*. Si marcas
«Borrar los datos de la aplicación», también se eliminan ajustes, historial y
modelos.

## Desarrollo

Requisitos: Rust estable (MSVC en Windows), Node.js 22+, y en Windows las
*Build Tools* de Visual Studio 2022 con C++. La primera compilación descarga las
bibliotecas estáticas de sherpa-onnx (`SHERPA_ONNX_ARCHIVE_DIR` permite usar
una copia local).

```bash
npm install
npm run app:dev        # aplicación completa en modo desarrollo
npm run app:build      # instaladores NSIS y MSI en target/release/bundle/
npm run dev            # solo la interfaz en el navegador (backend simulado)
```

Pruebas y comprobaciones:

```bash
cargo test --workspace                         # 139 pruebas
cargo clippy --workspace --all-targets -- -D warnings
npm run check                                  # tipos de la interfaz (svelte-check)
WISPR_TEST_MODELS=/ruta/modelos cargo test -p wispr-core --release --test e2e_local
```

En Windows, el ejecutable final incluye una autoprueba y hay un script de
medición de consumo:

```powershell
wispr-swag.exe --selftest --report informe.json `
  --model C:\ruta\sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8 `
  --audio C:\ruta\audio.wav --expect "texto esperado"
pwsh tools/measure.ps1 -Exe target\release\wispr-swag.exe
```

La vista previa en el navegador admite `?firstRun=1` (asistente),
`?lang=en` y `?noModel=1`.

## Estructura

| Carpeta | Contenido |
|---|---|
| `crates/wispr-core` | Núcleo sin interfaz: audio (captura, DSP, VAD), motores de voz local y en la nube, catálogo y descarga de modelos, procesamiento de texto, modos, vocabulario, IA, ajustes, historial (SQLite), credenciales y medición. |
| `crates/wispr-pill` | Dibujo de la barra flotante (tiny-skia + fontdue), sin dependencias de ventanas. |
| `src-tauri` | Aplicación de escritorio: controlador del dictado, capa Win32 (gancho de teclado, ventana por capas, inserción, portapapeles), bandeja, comandos IPC y autoprueba. |
| `src` | Interfaz en Svelte 5: inicio, historial, vocabulario, fragmentos, modos, aplicaciones, comandos y ajustes. |
| `tools` | Medición de consumo y generación de iconos. |
| `docs` | [Arquitectura](docs/ARCHITECTURE.md), [Privacidad](docs/PRIVACY.md), [Pruebas](docs/TESTING.md) y [Rendimiento](docs/PERFORMANCE.md). |

## Datos y privacidad

- Ajustes: `%APPDATA%\WisprSwag` (con copias de seguridad diarias).
- Historial, modelos y registros: `%LOCALAPPDATA%\WisprSwag`.
- Claves de API: Administrador de credenciales de Windows; nunca en archivos de
  ajustes ni en los registros.
- El audio no se guarda salvo que lo actives; los registros nunca contienen
  audio, textos ni claves.

Detalles en [docs/PRIVACY.md](docs/PRIVACY.md).

## Licencias de terceros

sherpa-onnx (Apache-2.0), ONNX Runtime (MIT), NVIDIA Parakeet TDT 0.6B v3
(CC-BY-4.0), OpenAI Whisper (MIT), Tauri (MIT/Apache-2.0), Svelte (MIT),
tiny-skia (BSD-3-Clause), fontdue (MIT/Apache-2.0/Zlib), tipografía Inter
(SIL OFL 1.1). La licencia del propio proyecto está pendiente de decisión por
su autor.
