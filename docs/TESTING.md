# Pruebas

## Automáticas

```bash
cargo test --workspace                                  # 139 pruebas
cargo clippy --workspace --all-targets -- -D warnings
npm run check                                           # svelte-check
WISPR_TEST_MODELS=/ruta/modelos \
  cargo test -p wispr-core --release --test e2e_local -- --nocapture
```

| Área | Pruebas | Qué cubren |
|---|---|---|
| Texto | 53 | Puntuación dictada y su contexto («una nueva línea de productos» no es un comando), correos dictados, muletillas, repeticiones intencionadas («muy muy»), arranques en falso, filtro de frases inventadas por el modelo, vocabulario y corrección fonética, sustituciones con expresiones regulares, fragmentos y variables de fecha, comandos de voz, modos (párrafos, viñetas, correo, código) y la canalización completa. |
| Atajos | 18 | Máquina de estados del gancho: mantener, toque corto, anulación con otra tecla, crecimiento a modo comando, Esc solo durante el dictado, pausa, modificador derecho en solitario, captura de atajos nuevos, enmascarado de Win/Alt/Mayús; formato y conflictos de combinaciones. |
| Audio | 11 | Remuestreo, filtro, normalización, VAD y segmentación, WAV, dispositivos. |
| Motores de voz | 11 | Catálogo, descarga verificada, división de audios largos, indicación al servicio en la nube, detección del formato NeMo. |
| IA | 12 | Indicaciones con protección frente a inyección, respuesta de Anthropic (incluida la negativa), API compatible con OpenAI, límites de uso y coste. |
| Datos | 19 | Ajustes (migración, saneamiento, copias), historial (búsqueda sin acentos, favoritos, retención, exportación), credenciales, perfiles por aplicación, idioma, medición del proceso, versiones. |
| Barra flotante | 12 | Distribución, animación y galería de capturas de todos los estados en tema claro y oscuro con medición del coste por fotograma. |
| Extremo a extremo | 3 | WAV real → audio preparado → Parakeet → texto final; silencio sin texto inventado; liberación del modelo (requiere el modelo descargado). |

La galería de la barra se genera en `target/pill-gallery/` con
`cargo test -p wispr-pill --test gallery`.

## Integración continua

`.github/workflows/ci.yml` se ejecuta en cada envío:

- **Linux** (`ubuntu-24.04`): tipos de la interfaz, compilación, clippy sin
  avisos, pruebas y prueba de extremo a extremo con Parakeet V3 real.
- **Windows** (`windows-latest`): pruebas, instalador NSIS y MSI, autoprueba
  del ejecutable final (con transcripción real en español e inglés), medición
  de consumo con `tools/measure.ps1`, e instalación y desinstalación
  silenciosas del instalador. Los resultados aparecen en el resumen de la
  ejecución y como artefactos (`wispr-swag-windows`, `informes-windows`).

RESULTADOS_WINDOWS

## Autoprueba en un equipo con Windows

```powershell
wispr-swag.exe --selftest --report informe.json
wispr-swag.exe --selftest --model C:\modelos\sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8 `
  --audio C:\audio\prueba.wav --expect "texto que se dice en el audio"
```

Comprueba: portapapeles (guardar, sustituir y restaurar), una ventana con un
campo de texto real donde inserta texto por pegado y por escritura Unicode
(acentos, ñ, €, comillas y emojis), la detección de la ventana activa, que la
barra flotante se muestra sin robar el foco y se cierra, el gancho de teclado
(Ctrl + Win mantenido sin abrir Inicio, anulación con otra tecla, modo comando,
Esc, Alt + Mayús + Z sin cambiar el idioma del teclado, captura y pausa) y, con
`--model`, la carga, la transcripción y la liberación de memoria. Nunca graba
el micrófono ni usa la red. Con `--ci` también prueba el inicio con Windows
(modifica la clave `Run` del usuario y la deja como estaba).

## Comprobación manual en Windows

Lo que una máquina de integración continua no puede probar (sin micrófono ni
usuario real):

1. **Primer arranque**: asistente completo; descarga del modelo con la barra de
   progreso; cancelar y reanudar la descarga.
2. **Micrófono**: medidor de nivel, «Grabar y transcribir» de 5 s, cambio de
   micrófono en caliente, desconexión durante un dictado (aviso y texto
   conservado para reintentar).
3. **Atajo en aplicaciones reales**: Bloc de notas, Word, Outlook, Chrome/Edge
   (Gmail, Google Docs), Slack, Teams, WhatsApp, VS Code, Windows Terminal y un
   programa abierto como administrador (el texto debe quedar en el portapapeles
   con aviso).
4. **Modos**: mantener, toque corto (manos libres) y Esc; crecimiento a modo
   comando añadiendo Alt.
5. **Dictado largo** (2–5 min) con texto incremental en la barra y en la vista
   detallada.
6. **Barra**: varios monitores con distinto escalado (100 %, 150 %, 200 %),
   arrastrar y recolocar, ocultar en pantalla completa, tema claro y oscuro.
7. **Suspensión e hibernación**: los atajos siguen funcionando al volver
   (si no, «Reiniciar atajos»).
8. **IA** con una clave real: consentimiento, prueba de conexión, modos con IA,
   modo comando, reescribir selección, límites y fallo de red (texto local).
9. **Instalador**: instalación, actualización con la aplicación abierta (debe
   cerrarse sola), inicio con Windows y desinstalación con y sin borrar datos.
