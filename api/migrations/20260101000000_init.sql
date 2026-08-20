-- TarascaPay :: esquema inicial
-- Todo el dinero se guarda como enteros (centavos) para evitar errores de punto flotante.

CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- --------------------------------------------------------------------------
-- Usuarios
-- --------------------------------------------------------------------------
CREATE TABLE users (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email         TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    display_name  TEXT NOT NULL,
    avatar_url    TEXT,
    currency      CHAR(3) NOT NULL DEFAULT 'ARS',
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- El email se normaliza a minúsculas en la aplicación; el índice garantiza unicidad.
CREATE UNIQUE INDEX users_email_key ON users (email);

-- --------------------------------------------------------------------------
-- Categorías de gasto
-- --------------------------------------------------------------------------
CREATE TABLE categories (
    id       SERIAL PRIMARY KEY,
    slug     TEXT NOT NULL UNIQUE,
    name     TEXT NOT NULL,
    icon     TEXT NOT NULL,
    position INT  NOT NULL DEFAULT 0
);

INSERT INTO categories (slug, name, icon, position) VALUES
    ('general',       'General',        '🧾', 0),
    ('alojamiento',   'Alojamiento',    '🏨', 1),
    ('comida',        'Comida',         '🍽️', 2),
    ('transporte',    'Transporte',     '🚕', 3),
    ('vuelos',        'Vuelos',         '✈️', 4),
    ('supermercado',  'Supermercado',   '🛒', 5),
    ('entretenimiento','Entretenimiento','🎭', 6),
    ('salidas',       'Salidas',        '🍻', 7),
    ('compras',       'Compras',        '🛍️', 8),
    ('salud',         'Salud',          '💊', 9),
    ('servicios',     'Servicios',      '💡', 10),
    ('alquiler',      'Alquiler',       '🔑', 11),
    ('otros',         'Otros',          '📦', 12);

-- --------------------------------------------------------------------------
-- Grupos (viajes, casas compartidas, etc.)
-- --------------------------------------------------------------------------
CREATE TABLE groups (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name        TEXT NOT NULL,
    description TEXT,
    kind        TEXT NOT NULL DEFAULT 'trip'
                CHECK (kind IN ('trip', 'home', 'couple', 'event', 'other')),
    currency    CHAR(3) NOT NULL DEFAULT 'ARS',
    emoji       TEXT NOT NULL DEFAULT '🌎',
    invite_code TEXT NOT NULL UNIQUE,
    created_by  UUID NOT NULL REFERENCES users (id) ON DELETE RESTRICT,
    archived    BOOLEAN NOT NULL DEFAULT FALSE,
    start_date  DATE,
    end_date    DATE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE group_members (
    group_id  UUID NOT NULL REFERENCES groups (id) ON DELETE CASCADE,
    user_id   UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    role      TEXT NOT NULL DEFAULT 'member' CHECK (role IN ('owner', 'member')),
    joined_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (group_id, user_id)
);

CREATE INDEX group_members_user_idx ON group_members (user_id);

-- --------------------------------------------------------------------------
-- Gastos
--   group_id NULL  => gasto personal (sólo visible por su dueño)
--   group_id NOT NULL => gasto compartido del grupo
-- --------------------------------------------------------------------------
CREATE TABLE expenses (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    group_id     UUID REFERENCES groups (id) ON DELETE CASCADE,
    created_by   UUID NOT NULL REFERENCES users (id) ON DELETE RESTRICT,
    paid_by      UUID NOT NULL REFERENCES users (id) ON DELETE RESTRICT,
    description  TEXT NOT NULL,
    notes        TEXT,
    amount_cents BIGINT NOT NULL CHECK (amount_cents > 0),
    currency     CHAR(3) NOT NULL,
    category_id  INT NOT NULL REFERENCES categories (id),
    expense_date DATE NOT NULL DEFAULT CURRENT_DATE,
    split_type   TEXT NOT NULL DEFAULT 'equal'
                 CHECK (split_type IN ('equal', 'exact', 'percentage', 'shares')),
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at   TIMESTAMPTZ
);

CREATE INDEX expenses_group_idx ON expenses (group_id, expense_date DESC) WHERE deleted_at IS NULL;
CREATE INDEX expenses_personal_idx ON expenses (created_by, expense_date DESC)
    WHERE group_id IS NULL AND deleted_at IS NULL;

-- Reparto del gasto entre participantes.
--   share_cents  => cuánto le toca pagar a ese usuario (siempre calculado en el backend)
--   split_value  => valor crudo ingresado por el usuario según split_type
--                   exact => centavos | percentage => puntos básicos (100.00% = 10000) | shares => cantidad de partes
CREATE TABLE expense_splits (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    expense_id  UUID NOT NULL REFERENCES expenses (id) ON DELETE CASCADE,
    user_id     UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    share_cents BIGINT NOT NULL,
    split_value BIGINT NOT NULL DEFAULT 0,
    UNIQUE (expense_id, user_id)
);

CREATE INDEX expense_splits_user_idx ON expense_splits (user_id);

-- --------------------------------------------------------------------------
-- Adjuntos (fotos / comprobantes)
-- --------------------------------------------------------------------------
CREATE TABLE attachments (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    expense_id   UUID REFERENCES expenses (id) ON DELETE CASCADE,
    uploaded_by  UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    file_name    TEXT NOT NULL,
    stored_name  TEXT NOT NULL,
    mime_type    TEXT NOT NULL,
    size_bytes   BIGINT NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX attachments_expense_idx ON attachments (expense_id);

-- --------------------------------------------------------------------------
-- Pagos / liquidaciones entre miembros
-- --------------------------------------------------------------------------
CREATE TABLE settlements (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    group_id     UUID REFERENCES groups (id) ON DELETE CASCADE,
    from_user    UUID NOT NULL REFERENCES users (id) ON DELETE RESTRICT,
    to_user      UUID NOT NULL REFERENCES users (id) ON DELETE RESTRICT,
    amount_cents BIGINT NOT NULL CHECK (amount_cents > 0),
    currency     CHAR(3) NOT NULL,
    note         TEXT,
    created_by   UUID NOT NULL REFERENCES users (id) ON DELETE RESTRICT,
    settled_at   DATE NOT NULL DEFAULT CURRENT_DATE,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CHECK (from_user <> to_user)
);

CREATE INDEX settlements_group_idx ON settlements (group_id, settled_at DESC);

-- --------------------------------------------------------------------------
-- Comentarios sobre un gasto
-- --------------------------------------------------------------------------
CREATE TABLE comments (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    expense_id UUID NOT NULL REFERENCES expenses (id) ON DELETE CASCADE,
    user_id    UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    body       TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX comments_expense_idx ON comments (expense_id, created_at);

-- --------------------------------------------------------------------------
-- Registro de actividad del grupo
-- --------------------------------------------------------------------------
CREATE TABLE activity (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    group_id   UUID REFERENCES groups (id) ON DELETE CASCADE,
    actor_id   UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    kind       TEXT NOT NULL,
    payload    JSONB NOT NULL DEFAULT '{}'::JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX activity_group_idx ON activity (group_id, created_at DESC);
