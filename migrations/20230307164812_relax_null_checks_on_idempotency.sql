-- Dropping NOT NULL from response data so the idempotency handler can insert *before* completing
-- the first request and any subsequent requests will see the pending row and wait
ALTER TABLE idempotency ALTER COLUMN response_status_code DROP NOT NULL;
ALTER TABLE idempotency ALTER COLUMN response_body DROP NOT NULL;
ALTER TABLE idempotency ALTER COLUMN response_headers DROP NOT NULL;