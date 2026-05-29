#!/usr/bin/env bash
# Upsert KEY=value lines in a dotenv file without disturbing other keys.
# Sourced by setup-postgres-dev-databases.sh (and safe to reuse elsewhere).

upsert_dotenv_var() {
  local file="$1"
  local key="$2"
  local value="$3"

  mkdir -p "$(dirname "$file")"
  touch "$file"

  if grep -q "^${key}=" "$file" 2>/dev/null; then
    # Use a delimiter unlikely in postgres URLs.
    sed -i "s|^${key}=.*|${key}=${value}|" "$file"
  else
    printf '%s=%s\n' "$key" "$value" >>"$file"
  fi
}
