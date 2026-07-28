# Para - Asistente de Flujo de Trabajo de IDE Paralelo

Para permite a los desarrolladores ejecutar múltiples sesiones de Claude Code simultáneamente. Cada sesión opera en un worktree de Git aislado con su propia instancia de VS Code, permitiendo el desarrollo paralelo sin conflictos de fusión (merge conflicts).

## El Problema

Al usar Claude Code para el desarrollo, estás limitado a una tarea a la vez. Cambiar entre funcionalidades implica perder el contexto y esperar a que las tareas se completen secuencialmente.

## La Solución

Para orquesta múltiples sesiones de Claude Code en paralelo. Mientras un Claude trabaja en la autenticación, otro construye la API y un tercero crea la UI; todo simultáneamente en ventanas separadas de VS Code.

## Ejemplo Sencillo

```bash
# Iniciar múltiples sesiones de Claude Code para diferentes tareas
para start auth -p "implement user authentication"    # Abre VS Code con Claude trabajando en auth
para start api -p "create REST API endpoints"        # Abre otro VS Code con Claude en la API

# Cada sesión de Claude:
# - Trabaja en su propia ventana de VS Code
# - Tiene su propia rama de Git y archivos aislados
# - Puedes editar código, revisar diffs y guiar a Claude
# - Acceso total a la terminal para ejecutar pruebas, etc.

# Cuando Claude completa una tarea:
para finish "Add authentication"  # Crea la rama de funcionalidad localmente para revisión
```

## Características Principales
- **Ejecución Paralela**: Ejecuta múltiples instancias de Claude Code concurrentemente en ventanas separadas de VS Code.
- **Control del Desarrollador**: Acceso directo al sistema de archivos, terminal y estado de Git de cada sesión.
- **Aislamiento**: Los worktrees de Git evitan conflictos entre tareas de desarrollo paralelo.
- **Monitoreo**: Panel en tiempo real que muestra el progreso de las sesiones, resultados de pruebas y bloqueos.
- **Intervención**: Entra en cualquier sesión para depurar, guiar a Claude o completar tareas manualmente.

## Videos de Demostración

**Creación de Sesiones Paralelas (0:03)**  
Muestra al Claude orquestador creando y despachando tres tareas de desarrollo paralelas a través de MCP.

https://github.com/user-attachments/assets/9edb6a12-c81b-4221-b42a-18fb3f8e2df7

**Panel de Monitoreo en Tiempo Real (0:04)**  
Demuestra el monitor de Para rastreando múltiples sesiones activas y su progreso.

https://github.com/user-attachments/assets/5737e21d-ab96-428c-87d3-683657f8d5c0

## Cómo Funciona Para

1. **Creación de Sesión**: Cada `para start` crea un worktree de Git aislado con su propia rama.
2. **Integración con Claude**: Claude Code se inicia en una nueva ventana de VS Code con la tarea especificada.
3. **Ejecución Paralela**: Múltiples sesiones se ejecutan independientemente sin conflictos de archivos.
4. **Acceso del Desarrollador**: Acceso total a la terminal, Git y al sistema de archivos en cada ventana de VS Code.
5. **Monitoreo**: Un panel en tiempo real rastrea todas las sesiones activas mediante `para monitor`.
6. **Finalización**: `para finish` crea ramas de funcionalidad a partir del trabajo completado.

## Instalación

```bash
brew install 2mawi2/tap/para
```

Para compilar desde el código fuente, consulta la [Guía de Desarrollo](docs/DEVELOPMENT.md).

## Primeros Pasos

```bash
# Configurar Claude Code como tu IDE
para config

# Habilitar la integración MCP en tu proyecto
para mcp init

# Iniciar tu primera sesión paralela
para start backend -p "create REST API with Express"

# Iniciar otra sesión (se abre en una nueva ventana de VS Code)
para start frontend -p "build React dashboard"

# Monitorear todas las sesiones
para monitor
```



## Orquestando el Desarrollo Paralelo

### Usando a Claude como tu Orquestador de Desarrollo

Trabaja con una sesión de Claude Code para coordinar todo tu proyecto. Claude tiene conocimiento integrado de los comandos de Para a través de la integración MCP, lo que le permite:

- Desglosar funcionalidades complejas en tareas paralelas.
- Iniciar nuevas sesiones de Claude para cada tarea.
- Monitorear el progreso en todas las sesiones.
- Ayudar a revisar e integrar el trabajo completado.

### Ejemplo: Construyendo una Aplicación Web

```
Desglose de Tareas:
1. Esquema de base de datos (debe completarse primero)
2. Endpoints de la API (depende del esquema)
3. UI del Frontend (puede iniciar con la API)
4. Autenticación (paralelo a la UI)
5. Pruebas de integración (después de que todo termine)

Ejecución Paralela:
- Sesión 1: Base de datos → Endpoints de la API
- Sesión 2: UI del Frontend
- Sesión 3: Autenticación
- Sesión 4: Pruebas (inicia más tarde)
```

### Automatizando tu Flujo de Trabajo

Crea comandos slash personalizados de Claude para adaptarlos al proceso de desarrollo de tu equipo:

```bash
# Ejemplos de comandos en .claude/commands/
/breakdown   # Analiza la funcionalidad y crea tareas paralelas
/review      # Revisa todas las ramas completadas
/integrate   # Sugiere el orden de fusión (merge)

# Personaliza para tu flujo de trabajo:
- Creación de PR vs ramas locales
- Convenciones de nomenclatura de ramas
- Requisitos de revisión
- Estrategia de integración
```

La idea clave: Con múltiples sesiones de Claude trabajando en paralelo, la revisión de código se convierte en la restricción principal. La automatización ayuda a gestionar este aumento de rendimiento.

## Consideraciones de Seguridad

### Ejecución Autónoma

Cuando ejecutes Claude con `--dangerously-skip-permissions` (omite las solicitudes de permiso del IDE), usa sandboxing para mayor seguridad:

```bash
# macOS: Restringir escrituras de archivos al directorio de la sesión
para start -p "build feature" --dangerously-skip-permissions --sandbox

# macOS: Restringir también el acceso a la red  
para start -p "refactor code" --dangerously-skip-permissions --sandbox-no-network

# Todas las plataformas: Aislamiento total con Docker
para start -p "risky task" --dangerously-skip-permissions --container
```

Para una configuración detallada de sandboxing y opciones de seguridad, consulta la [Guía de Sandboxing](docs/SANDBOXING.md).

### Notas Importantes

- **Revisión de Código**: Revisa siempre el código generado antes de fusionarlo.
- **Sandboxing**: Recomendado al usar `--dangerously-skip-permissions`.
- **Descargo de Responsabilidad**: Los usuarios son responsables de revisar y validar todo el código generado por la IA.


## Cuándo Usar Para

Para está diseñado para desarrolladores que desean aprovechar múltiples sesiones de Claude Code para un desarrollo más rápido:

- **Desarrollo de Funcionalidades**: Ejecuta sesiones paralelas de Claude para frontend, backend y pruebas.
- **Refactorización a Gran Escala**: Divide la refactorización entre módulos con múltiples instancias de Claude.  
- **Corrección de Errores**: Un Claude corrige errores mientras otro añade funcionalidades.
- **Experimentación**: Puedes pedirle a Claude que ejecute múltiples sesiones de Para sobre el mismo archivo de plan y conservar el mejor resultado.

Sin Para, estás limitado a una sesión de Claude Code a la vez. Con Para, puedes orquestar todo un equipo de instancias de Claude.

## Integración de IA

### Integración MCP

Para se integra con Claude Code a través de MCP (Model Context Protocol). El flujo de trabajo típico es:

1. **Planificación del Desarrollo** - Trabaja con Claude para crear planes de implementación detallados.
2
2. **Guardar Planes** - Escribe estos planes en archivos .md (donde prefieras).
3. **Ejecutar en Paralelo** - Claude inicia sesiones de Para a partir de estos archivos de plan.

Ejemplos de prompts para el flujo de trabajo:

**Fase de Planificación:**
- "Planifiquemos cómo implementar la autenticación de usuarios. Escribe un plan sencillo que cubra la base de datos, la API y el frontend"
- "Desglosa esta integración de pagos en tareas independientes que puedan desarrollarse en paralelo"
- "Crea un plan de implementación de MVP sencillo para esta funcionalidad que evite conflictos de fusión entre componentes"

**Mejor Práctica:** Comienza con planes sencillos centrados en el MVP. Evita la sobreingeniería en las sesiones de agentes; siempre puedes iterar y expandir más tarde.

**Guardar Planes** (tú eliges dónde):
- "Escribe este plan en `tasks/auth-implementation.md`"
- "Guarda el plan de cada componente en archivos separados en `planning/`" 
- "Crea archivos de tareas numerados: `TASK_1_database.md`, `TASK_2_api.md`"

**Ejecutar Planes:**
- "Inicia una sesión de Para a partir del plan de implementación de autenticación que acabamos de crear"
- "Lanza sesiones de Para para cada uno de los archivos de tareas en el directorio de planificación"
- "Comienza el desarrollo paralelo usando los planes en `tasks/TASK_*.md`"

La clave es que **tú y Claude acuerden los detalles de implementación primero**, y luego Para ejecute estos planes bien definidos en paralelo. Esto limita el grado de libertad de cada Agente de Claude y reduce la probabilidad de YAGNI y reward hacking.

Nota: Puedes usar cualquier herramienta de planificación que prefieras: archivos .md locales, issues de GitHub o herramientas como [claude-task-master](https://github.com/eyaltoledano/claude-task-master). Para solo necesita acceso para leer los archivos de plan.

## Ejemplo Completo: Construyendo una App de Tareas (Todo App)

Así es como se ve el flujo de trabajo completo en la práctica:

1. **Planificar con Claude**: "Construyamos una app de tareas. Crea un plan sencillo que cubra la API del backend, la UI del frontend y la configuración de la base de datos"

2. **Claude crea archivos de plan**:
   ```
   tasks/todo-backend.md   - API REST con Express
   tasks/todo-frontend.md  - Componentes de UI en React  
   tasks/todo-database.md  - Esquema de PostgreSQL
   ```

3. **Iniciar sesiones paralelas**: "Lanza sesiones de Para para cada uno de estos archivos de tareas"
   - Claude ejecuta `para_start` para cada archivo de plan.
   - Se abren 3 ventanas de VS Code, cada una con Claude trabajando en un componente diferente.

4. **Monitorear el progreso**: Ejecutas `para monitor` o simplemente `para` para observar todas las sesiones.
   - Mira qué sesiones están programando, probando o bloqueadas.
   - Actualizaciones en tiempo real sobre el progreso y resultados de las pruebas.

5. **Intervenir cuando sea necesario**: La sesión de frontend tiene problemas de estilo.
   - Entra en esa ventana de VS Code: `cd .para/worktrees/todo-frontend` (o reanuda desde el monitor de para).
   - Corrige el CSS, guía a Claude, ejecuta pruebas localmente.
   - Continúen el desarrollo juntos.

6. **Completar e integrar**: Las sesiones terminan su trabajo.
   - Cada una crea una rama de funcionalidad: `para finish "Add todo backend"`.
   - Revisas las 3 ramas y las fusionas.
   - La app de tareas completa está lista.

Este flujo de trabajo te permite construir aplicaciones complejas entre 3 y 5 veces más rápido manteniendo el control total.

## Comandos Esenciales

```bash
# Iniciar nuevas sesiones
para start backend -p "create REST API"           # Nueva sesión de IA  
para start --file tasks/auth-plan.md              # Desde archivo de plan
para start frontend                               # Sesión manual

# Monitorear y controlar
para monitor                                      # Panel en tiempo real
para list                                         # Mostrar sesiones activas  
para finish "Add user authentication"            # Completar sesión
para config                                      # Configurar IDE
```

Para una referencia completa de comandos, consulta la [Documentación de la CLI](docs/CLI_REFERENCE.md).

## Ejemplos de Flujos de Trabajo

### Desarrollo Orquestado (Recomendado)
```bash
# 1. Planificar con Claude
"Desglosa la autenticación de usuarios en tareas paralelas"
# Claude crea: tasks/auth-api.md, tasks/auth-ui.md, tasks/auth-tests.md

# 2. Ejecutar plan  
"Inicia sesiones de Para para cada archivo de tarea de auth"
# Se abren 3 ventanas de VS Code con Claude trabajando

# 3. Monitorear y guiar
para monitor              # Observar progreso
# Entra en las sesiones según sea necesario para ayudar
```

### Gestión de Sesiones Manual  
```bash
# Iniciar sesiones individuales de Claude directamente
para start auth -p "implement JWT authentication"
para start ui -p "create login form"
para start tests -p "add auth unit tests"

# O iniciar sesiones manuales (sin prompt de IA)
para start feature-payments    # Trabajas directamente en VS Code
para start bugfix-login       # Sesión de depuración manual
```

Ambos enfoques utilizan worktrees de Git aislados para evitar conflictos.

## Características Avanzadas

- **Nombres de Rama Personalizados**: `para finish "mensaje" --branch nombre-personalizado`
- **Recuperación de Sesiones**: Copia de seguridad automática de las últimas 3 sesiones canceladas.  
- **Reanudar con Contexto**: Añade instrucciones al reanudar sesiones.
- **Integración de IDE**: Funciona con Claude Code, VS Code, Cursor y otros.

Para configuración detallada y uso avanzado, consulta la sección de [Documentación](#documentation).


## Documentación

### Primeros Pasos
- **[Referencia de la CLI](docs/CLI_REFERENCE.md)** - Documentación completa de comandos.
- **[Guía de Autocompletado del Shell](docs/SHELL_COMPLETION.md)** - Configuración y ajuste del autocompletado del shell.
- **[Configuración Detallada](docs/DETAILED_CONFIGURATION.md)** - Configuración avanzada del IDE y variables de entorno.

### Integración de IA  
- **[Integración MCP](docs/MCP_INTEGRATION.md)** - Guía completa de integración MCP con Claude Code.
- **[Orquestación MCP](docs/MCP_ORCHESTRATION.md)** - Patrones y flujos de trabajo de orquestación de IA.
- **[Instrucciones de Ejemplo](docs/SAMPLE_PARA_INSTRUCTIONS.md)** - Ejemplo de CLAUDE.md para flujos de trabajo basados en CLI.

### Seguridad y Avanzado
- **[Guía de Sandboxing](docs/SANDBOXING.md)** - Configuración completa de sandboxing y opciones de seguridad.
- **[Guía de Flujo de Trabajo](docs/WORKFLOW.md)** - Diagramas visuales de los flujos de trabajo y transiciones de estado de Para.
- **[Comportamiento del IDE](docs/IDE_BEHAVIOR.md)** - Gestión de ventanas del IDE durante la integración y conflictos.

### Soporte
- **[Solución de Problemas](docs/TROUBLESHOOTING.md)** - Problemas comunes y soluciones.  
- **[Guía de Desarrollo](docs/DEVELOPMENT.md)** - Contribución, arquitectura, pruebas.

## Requisitos

- Git 2.5+ (para soporte de worktree)
- Tu IDE preferido con soporte para CLI

## Cómo Ser Productivo

1. **Instalar**: `brew install 2mawi2/tap/para`
2. **Configurar**: `para config` (configura tu IDE)
3. **Habilitar MCP**: `para mcp init` (para la integración con Claude Code)
4. **Empezar a construir**: Usa el flujo de planificación descrito anteriormente.

Para el autocompletado del shell: `para init`

## Configuración Avanzada

Para soporta variables de entorno, automatización de CI y personalización detallada. Consulta los enlaces de documentación a continuación para ver todas las opciones de configuración.
