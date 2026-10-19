-- Employees of this tenant, what each one may do, and their sign-in sessions.
-- This database belongs to a single tenant, so nothing here carries a
-- tenant_id.

CREATE TABLE employees (
    id            uuid        PRIMARY KEY DEFAULT gen_random_uuid(),
    email         text        NOT NULL UNIQUE CHECK (email = lower(email)),
    password_hash text        NOT NULL,
    full_name     text        NOT NULL,
    -- The first employee. Implicitly allowed everything; never stored as
    -- permission rows.
    is_executive  boolean     NOT NULL DEFAULT false,
    is_active     boolean     NOT NULL DEFAULT true,
    created_at    timestamptz NOT NULL DEFAULT now(),
    updated_at    timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX employees_single_executive ON employees (is_executive) WHERE is_executive;

-- One row per (employee, resource). A missing row means no access at all.
CREATE TABLE employee_permissions (
    employee_id uuid    NOT NULL REFERENCES employees (id) ON DELETE CASCADE,
    resource    text    NOT NULL,
    can_create  boolean NOT NULL DEFAULT false,
    can_read    boolean NOT NULL DEFAULT false,
    can_update  boolean NOT NULL DEFAULT false,
    can_delete  boolean NOT NULL DEFAULT false,
    PRIMARY KEY (employee_id, resource)
);

CREATE TABLE auth_sessions (
    token_hash       text        PRIMARY KEY,
    employee_id      uuid        NOT NULL REFERENCES employees (id) ON DELETE CASCADE,
    user_agent       text,
    created_at       timestamptz NOT NULL DEFAULT now(),
    last_activity_at timestamptz NOT NULL DEFAULT now(),
    expires_at       timestamptz NOT NULL
);

CREATE INDEX auth_sessions_employee_id_idx ON auth_sessions (employee_id);
CREATE INDEX auth_sessions_expires_at_idx ON auth_sessions (expires_at);
