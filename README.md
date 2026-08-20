# TarascaPay

TarascaPay es una aplicación open source para dividir gastos: pensada para viajes, casas
compartidas y el día a día. Es una alternativa auto-hospedable y enfocada en la privacidad
a Splitwise.

- **Backend:** Rust + Axum + PostgreSQL
- **Frontend:** SPA en React + TypeScript
- **Todo el dinero se maneja en centavos enteros** — nunca en punto flotante.

---

## Qué hace

**Grupos**
- Viajes, casas compartidas, parejas, eventos. Cada grupo tiene su propia moneda.
- Se suma gente por email o con un código de invitación de 8 caracteres.
- Roles: quien crea el grupo es administrador.
- Historial de actividad de todo lo que pasa en el grupo.

**Gastos**
- Cuatro formas de repartir: partes iguales, importes exactos, porcentajes y por partes
  (ej. una pareja pone el doble que alguien que va solo).
- Quien pagó puede ser cualquier miembro, no sólo quien carga el gasto.
- 13 categorías precargadas, notas y fecha propia del gasto.
- Comentarios por gasto.
- Borrado lógico: eliminar un gasto no rompe el historial.

**Comprobantes**
- Fotos y PDFs adjuntos a cada gasto (JPG, PNG, WebP, GIF, HEIC, PDF).
- El tipo de archivo se valida por los bytes reales, no por lo que declare el cliente.
- Sólo los ven quienes tienen acceso al gasto.

**Balances**
- Saldo neto por persona dentro de cada grupo.
- **Simplificación de deudas:** en vez de N·(N-1)/2 pagos cruzados, sugiere como mucho
  N-1 transferencias para que todos queden a mano.
- Registro de pagos hechos, con historial.

**Gastos personales**
- Gastos propios que no se comparten con nadie. Sólo los ve su dueño.

**Panel y estadísticas**
- Cuánto te deben, cuánto debés y con quién.
- Gasto del mes, personal y en grupos.
- Desglose por categoría y evolución mes a mes.

**Otros**
- Sesión con JWT, contraseñas con Argon2.
- Tema claro/oscuro (por defecto sigue al sistema).
- Interfaz responsive: barra lateral en escritorio, barra inferior en móvil.

---

## Cómo correrlo

### Opción A — Docker (todo junto)

```bash
docker compose -f docker/docker-compose.yml up --build
```

La app queda en <http://localhost:3000>. La API sirve también el SPA compilado, así que
no hay que levantar nada más.

### Opción B — Desarrollo local

**1. Base de datos.** Con Docker:

```bash
docker compose -f docker/docker-compose.yml up postgres_db -d
```

O con un PostgreSQL propio:

```bash
createdb tarascapay_db
```

**2. Variables de entorno.**

```bash
cp .env.example .env
# ajustá DATABASE_URL y poné un JWT_SECRET propio
```

**3. API.** Las migraciones se aplican solas al arrancar:

```bash
cargo run -p api
# escuchando en http://127.0.0.1:3000
```

**4. Frontend.** En otra terminal:

```bash
cd web
npm install
npm run dev
# http://localhost:5173 (con proxy hacia la API)
```

### Build de producción

```bash
cd web && npm run build && cd ..
WEB_DIST=./web/dist cargo run --release -p api
```

Con `WEB_DIST` apuntando al build, un solo proceso sirve la API y el frontend.

---

## Variables de entorno

| Variable           | Por defecto               | Para qué sirve                                         |
| ------------------ | ------------------------- | ------------------------------------------------------ |
| `DATABASE_URL`     | _(obligatoria)_           | Cadena de conexión a PostgreSQL                        |
| `HOST`             | `127.0.0.1`               | Interfaz de escucha (`0.0.0.0` en contenedor)          |
| `PORT`             | `3000`                    | Puerto HTTP                                            |
| `JWT_SECRET`       | _(efímero)_               | Firma de los tokens. **Definilo en producción**        |
| `JWT_TTL_HOURS`    | `720`                     | Duración de la sesión                                  |
| `UPLOAD_DIR`       | `./uploads`               | Dónde se guardan los comprobantes                      |
| `MAX_UPLOAD_BYTES` | `10485760`                | Tamaño máximo por archivo (10 MB)                      |
| `CORS_ORIGINS`     | `http://localhost:5173,…` | Orígenes permitidos, separados por coma. `*` abre todo |
| `WEB_DIST`         | —                         | Si apunta al build del SPA, la API lo sirve            |
| `RUST_LOG`         | `api=info,…`              | Nivel de logs                                          |

Sin `JWT_SECRET` la app arranca igual, pero genera un secreto al azar: todas las sesiones
se invalidan en cada reinicio. Está bien para desarrollo, no para producción.

---

## Tests

```bash
# Backend: lógica de reparto, simplificación de deudas, auth, validaciones
cargo test

# Frontend: chequeo de tipos + build
cd web && npm run build

# API completa contra un servidor corriendo (necesita jq)
./scripts/smoke-test.sh
```

`scripts/smoke-test.sh` recorre la API entera de punta a punta: registro, permisos, los
cuatro tipos de reparto, balances, liquidaciones, comprobantes y estadísticas.

---

## Estructura

```
api/          Servidor Axum
  migrations/   Migraciones SQL (se aplican al arrancar)
  src/
    auth.rs       JWT + Argon2 + extractor de usuario
    config.rs     Configuración por entorno
    error.rs      Error unificado → respuesta HTTP
    models.rs     Filas de la base y su mapeo a DTOs
    repo.rs       Consultas compartidas (balances, gastos)
    validate.rs   Validación de entrada
    routes/       auth, groups, expenses, settlements, attachments, dashboard
shared/       DTOs y la lógica de negocio pura
  src/split.rs    Reparto de gastos y simplificación de deudas (con tests)
web/          SPA en React + TypeScript + Vite + Tailwind
  src/
    lib/          Cliente de API, tipos, formato, sesión, tema
    components/   UI reutilizable y formulario de gastos
    pages/        Pantallas
scripts/      Utilidades (smoke test de la API)
docker/       Dockerfile y docker-compose
```

---

## Decisiones de diseño

**El dinero es siempre un entero.** Todos los importes se guardan y viajan en centavos.
El reparto usa el método del _resto mayor_: se asignan las partes enteras y los centavos
sobrantes van a quienes tienen el resto fraccionario más grande. La suma de las partes
coincide siempre con el total, al centavo. Está cubierto por tests en
`shared/src/split.rs`.

**Un gasto personal es un gasto de un solo participante.** Así balances, estadísticas y
listados lo tratan igual que a cualquier otro, sin código aparte.

**Los saldos se calculan, no se guardan.** Un saldo desnormalizado se desincroniza en
cuanto se edita o borra un gasto. Las agregaciones se hacen en PostgreSQL.

**Consultas en runtime, no macros de sqlx.** Se usa `query_as` en vez de `query!` para que
compilar no dependa de tener una base de datos disponible.

**Multi-moneda: parcial.** Cada grupo tiene su moneda y ahí los números son exactos. El
panel principal suma los saldos de todos tus grupos sin convertir monedas — si manejás
grupos en monedas distintas, el detalle fiel está en la pantalla de cada grupo.

---

## Estado del proyecto

Funciona de punta a punta y está probado, pero le faltan cosas para un despliegue serio:

- No hay recuperación de contraseña por email.
- No hay límite de intentos de login (rate limiting).
- Los comprobantes se guardan en disco local, no en almacenamiento de objetos.
- No hay conversión entre monedas.
- No hay gastos recurrentes ni notificaciones.

---

## Sobre el código

Este proyecto arrancó con una regla de sólo-código-humano. Esa regla ya no aplica: la
versión actual fue construida con asistencia de IA (Claude). Queda escrito acá para que no
confunda a quien llegue al repositorio.

Lo que sí se mantiene es el estándar: la lógica de negocio tiene tests, la API está
verificada de punta a punta y las decisiones de diseño están documentadas arriba.

---

## Licencia

MIT — ver [LICENSE](LICENSE).
