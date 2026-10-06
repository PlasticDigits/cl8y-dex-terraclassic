-- Bootstrap only the administrator database. Runtime/migrator/QA roles and databases
-- are provisioned by scripts/setup-postgres-dev-databases.sh with explicit secrets.
-- No application login is a superuser; no test DB inherits unrestricted PUBLIC ACLs.
REVOKE CONNECT ON DATABASE postgres FROM PUBLIC;
REVOKE CONNECT ON DATABASE template1 FROM PUBLIC;
REVOKE ALL ON SCHEMA public FROM PUBLIC;
