# Privacidad y seguridad

Principio general: **todo ocurre en tu equipo salvo que tú actives lo
contrario**, y cuando se activa una conexión se explica antes qué se envía.

## Qué se graba y cuándo

- El micrófono solo se abre mientras dictas (la barra muestra «escuchando» y la
  forma de onda) o durante las pruebas de micrófono de los ajustes, que son
  visibles y se detienen solas a los 60 s o al cerrar la ventana.
- No hay escucha en segundo plano ni palabra de activación.
- El audio se procesa en memoria. **No se guarda por defecto.** Si lo activas en
  *Privacidad › Guardar el audio de los dictados*, se guarda como WAV en tu
  equipo y se borra automáticamente pasado el plazo elegido (1, 7 o 30 días).
- Si una transcripción falla, el audio se conserva solo en memoria durante 10
  minutos para poder pulsar «Reintentar», y después se descarta.

## Qué se guarda en el equipo

| Dato | Dónde | Control |
|---|---|---|
| Ajustes, modos, vocabulario, fragmentos y perfiles | `%APPDATA%\WisprSwag\settings.json` (+ copias de seguridad) | Exportar, importar y restablecer. Las claves nunca se incluyen. |
| Historial de dictados | `%LOCALAPPDATA%\WisprSwag\history.sqlite3` | Activable; retención de 1 día a siempre (90 días por defecto); los favoritos se conservan; búsqueda, edición, exportación y borrado. |
| Texto original sin procesar | Dentro del historial | Desactivable. |
| Nombre de la aplicación de destino | Dentro del historial | Solo el ejecutable (`outlook.exe`), nunca el título de la ventana. Desactivable. |
| Estadísticas | Dentro del historial | Recuentos por día y por aplicación, sin texto. |
| Modelos de voz | `%LOCALAPPDATA%\WisprSwag\models` | Descarga y borrado desde *Motor de voz*. |
| Registros de diagnóstico | `%LOCALAPPDATA%\WisprSwag\logs` | Rotación diaria, 7 archivos. **Nunca** contienen audio, transcripciones, textos seleccionados ni claves. |
| Uso de la IA | `%LOCALAPPDATA%\WisprSwag\ai_usage.json` | Solo contadores (solicitudes, tokens, coste estimado). |
| Claves de API | Administrador de credenciales de Windows (`WisprSwag/…`) | Nunca en archivos de ajustes, exportaciones ni registros. La interfaz solo ve una versión enmascarada. |

*Privacidad › Tus datos* muestra cuánto ocupa cada cosa y permite borrar por
separado historial, audio, estadísticas, contadores de IA, claves, modelos y
registros. Al desinstalar, la opción «Borrar los datos de la aplicación» elimina
también estas carpetas, y el inicio automático se quita siempre.

## Conexiones a Internet

Son todas las que puede hacer la aplicación, y cada una se ve en
*Privacidad › Conexiones a Internet*:

1. **Descarga de modelos de voz**: solo al pulsar «Descargar», desde las
   publicaciones de sherpa-onnx en GitHub, con verificación SHA-256.
2. **Transcripción en la nube**: solo si eliges el motor «Nube». El audio de
   cada dictado va directamente al proveedor configurado (OpenAI, Groq o tu
   servidor) con tu clave.
3. **IA**: solo si la activas y aceptas el aviso de consentimiento. Se envía el
   texto ya transcrito y limpiado, las instrucciones del modo y, si lo permites,
   el tipo genérico de aplicación («un cliente de correo») y los términos de tu
   vocabulario. En el modo comando y al reescribir, también el texto
   seleccionado. Nunca el audio, el historial, títulos de ventanas ni nombres de
   archivos. Con Ollama o LM Studio el texto no sale del equipo.
4. **Comprobación de actualizaciones**: desactivable; consulta la última versión
   publicada en GitHub sin enviar datos personales.

No hay telemetría, analíticas ni informes de errores automáticos.

## Protecciones técnicas

- **Inyección de instrucciones**: el dictado se envía a la IA entre etiquetas y
  con instrucciones explícitas de tratarlo como texto, no como órdenes. Si la
  respuesta tiene una longitud desproporcionada respecto al dictado (señal
  típica de que la IA ha contestado en lugar de editar), se descarta y se usa el
  texto local.
- **Límites**: solicitudes al día y tokens al mes, comprobados antes de cada
  envío; longitud máxima por solicitud; tiempo máximo de espera.
- **Portapapeles**: al pegar, el contenido anterior se restaura y el dictado se
  marca para que no aparezca en el historial del portapapeles de Windows ni se
  sincronice en la nube.
- **Aplicaciones con privilegios**: Windows no permite enviar pulsaciones a
  programas abiertos como administrador desde uno normal; en ese caso el texto
  se deja en el portapapeles y se avisa, sin pedir más permisos.
- **Sin elevación**: la aplicación se instala y funciona sin permisos de
  administrador.
- **Lista cerrada de acciones del sistema**: la interfaz solo puede abrir tres
  páginas concretas de la Configuración de Windows, las carpetas de la propia
  aplicación y enlaces web en el navegador.
- **Política de seguridad de contenido** restrictiva en la interfaz.
