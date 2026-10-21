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

Equipo: máquina virtual de GitHub Actions con Windows Server 2025 Datacenter
24H2 (compilación 26100), 2 núcleos lógicos, sin micrófono y con sesión de
administrador. Ejecutable final en modo *release* (ejecución 39516894158 de la
CI, 21 de octubre de 2026).

### Consumo de la aplicación (`tools/measure.ps1`)

Perfil limpio, inicio minimizado, sin modelo cargado. Muestras cada segundo.

| Estado | Memoria del proceso (conjunto de trabajo) | Memoria privada | Total con WebView2 | CPU del proceso | Procesos WebView2 |
|---|---|---|---|---|---|
| Reposo: bandeja, atajos y barra (60 s) | 23,1 MB | 9,0 MB | 23,1 MB | 0 % | 0 |
| Ventana principal abierta (20 s) | 38,7 MB | 11,2 MB | 338,5 MB | 0 % | 6 |
| Después de cerrar la ventana (60 s) | 28,4 MB | 11,1 MB | 28,4 MB | 0 % | 0 |

- La CPU se mide con el tiempo de procesador acumulado del proceso principal y
  se redondea a milésimas: en reposo no consumió tiempo de CPU medible en todo el
  minuto (Windows lo contabiliza en intervalos de unos 15 ms).
- Con la ventana abierta, la CPU de los procesos de WebView2 no está incluida en
  esa columna; su memoria sí está en «Total con WebView2».
- Al cerrar la ventana, los 6 procesos de WebView2 desaparecen y se recuperan
  unos 310 MB. La aplicación se cerró de forma ordenada con `--quit`.

### Motor de voz (autoprueba con el ejecutable final)

| Medida | Resultado |
|---|---|
| Carga de Parakeet V3 int8 (2 hilos) | 2,4 s |
| Memoria del proceso | 25 MB antes de cargar → 714 MB con el modelo → 763 MB al transcribir |
| Memoria tras liberar el modelo | 47,7 MB (se liberan 716 MB) |
| 5,3 s de voz en español | 1,08 s, texto exacto («No preguntes qué puede hacer tu país por ti, pregunta qué puedes hacer tú por tu país.») |
| 3,8 s de voz en inglés | 0,88 s, texto exacto |
| Fotograma de la barra | 0,35 ms (100 %), 0,54 ms (150 %), 0,49 ms (200 %) |

Con solo 2 núcleos virtuales la transcripción va unas 4,5–5 veces más rápida
que el tiempo real. En la máquina Linux (4 núcleos, 2 hilos para el motor) fue de
9 a 11 veces. En un equipo con 8 núcleos lógicos el motor usa 4 hilos; ese caso
no se ha medido.

Instaladores: NSIS y MSI, 39,6 MB los dos juntos comprimidos.

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
