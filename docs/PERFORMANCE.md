# Rendimiento y consumo

Todas las cifras de este documento están **medidas**, con el equipo y el estado
de la aplicación indicados. Para repetirlas:

- Windows: `pwsh tools/measure.ps1 -Exe ruta\wispr-swag.exe` (reposo, ventana
  abierta y ventana cerrada) y `wispr-swag.exe --selftest --model … --audio …`
  (carga del modelo, velocidad de transcripción y liberación de memoria).
- Linux: `wispr-swag --selftest --model … --audio …` (pruebas portables).

## Diseño orientado al consumo

| Situación | Qué está activo | Qué no |
|---|---|---|
| Reposo | Un proceso: bandeja, gancho de teclado (hilo bloqueado en `GetMessage`) y barra flotante (sin temporizador). | Sin WebView2, sin micrófono, sin modelo en memoria (salvo que se elija «Siempre cargado» o «Cargar al iniciar»), sin red. |
| Dictando | Micrófono, forma de onda en la barra (30 fps por defecto, 20 en bajo consumo) y transcripción por frases en cada pausa. | La ventana de ajustes no hace falta. |
| Tras dictar | El modelo sigue en memoria 10 min (configurable) y luego se libera y se devuelve la memoria al sistema. | — |
| Ventana de ajustes | Procesos de WebView2 mientras está abierta. | Al cerrarla se destruye (opción por defecto). |

## Windows

Mediciones de la integración continua de GitHub Actions (`windows-latest`).

PENDIENTE_WINDOWS

## Linux (pruebas portables)

Equipo: máquina virtual con Intel Xeon a 2,1 GHz, 4 núcleos lógicos y 16 GB de
RAM, sin GPU. Binario final en modo *release*. Motor: Parakeet TDT 0.6B v3 int8
con 2 hilos (la elección automática usa la mitad de los núcleos).

| Medida | Resultado |
|---|---|
| Carga del modelo | 2,4–2,5 s |
| Memoria del proceso con el modelo cargado | ≈0,8 GB (pico de 0,83 GB al transcribir) |
| Memoria tras liberar el modelo | ≈40 MB (ejemplo mínimo) / ≈80 MB (aplicación completa) |
| 5,3 s de voz en español | 0,51–0,58 s (≈9–10 veces más rápido que el tiempo real) |
| 14,2 s de voz en español | 1,31 s (≈11 veces más rápido que el tiempo real) |
| Fotograma de la barra | 0,13 ms (100 %), 0,26 ms (150 %), 0,34 ms (200 %) |

A 30 fps, dibujar la barra mientras se dicta cuesta menos del 1 % de un núcleo,
y en reposo no se dibuja nada.

Dos problemas detectados y corregidos con estas mediciones:

1. **Carga doble del modelo.** Si no se indica el tipo de modelo, sherpa-onnx lo
   detecta cargándolo, y la memoria subía a 1,48 GB y la carga a 4,7 s. Ahora se
   indica siempre (también en modelos importados, detectando el formato NeMo por
   su archivo de símbolos): 0,8 GB y 2,4 s.
2. **Memoria retenida al liberar.** El asignador de memoria conservaba unos
   600 MB tras descargar el modelo. Ahora se devuelve al sistema (`malloc_trim`
   en Linux, `HeapCompact` en Windows) y el proceso vuelve a su consumo base.

## Modelos disponibles

| Modelo | Descarga | Memoria al usarlo | Notas |
|---|---|---|---|
| Parakeet V3 (recomendado) | 465 MB | ≈0,8 GB | Rápido y muy preciso en español; puntuación y mayúsculas propias; no inventa texto en silencios. |
| Whisper Turbo | 538 MB | ≈1,6 GB | 99 idiomas, buena precisión, unas 5 veces más lento que Parakeet en CPU. |
| Whisper Small | 610 MB | ≈0,7 GB | Equilibrio para idiomas que Parakeet no cubre. |
| Whisper Base | 198 MB | ≈0,35 GB | Ligero, precisión limitada. |

La memoria de los modelos Whisper es la estimada en el catálogo; la de Parakeet
está medida.
