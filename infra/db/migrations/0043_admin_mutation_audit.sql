CREATE TABLE admin_mutation_audit (
    audit_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    actor TEXT NOT NULL CHECK (length(actor) BETWEEN 1 AND 128),
    role TEXT NOT NULL CHECK (role IN ('operator', 'admin')),
    method TEXT NOT NULL CHECK (method IN ('POST', 'PUT', 'PATCH', 'DELETE')),
    path TEXT NOT NULL CHECK (path LIKE '/admin/%' AND length(path) <= 512),
    response_status INTEGER CHECK (response_status IS NULL OR response_status BETWEEN 100 AND 599),
    request_id TEXT CHECK (request_id IS NULL OR length(request_id) <= 128),
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX admin_mutation_audit_occurred_at_idx
    ON admin_mutation_audit (occurred_at DESC);

CREATE INDEX admin_mutation_audit_actor_occurred_at_idx
    ON admin_mutation_audit (actor, occurred_at DESC);
