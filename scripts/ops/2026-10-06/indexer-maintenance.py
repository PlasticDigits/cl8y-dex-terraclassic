#!/usr/bin/env python3
"""Scoped indexer DB snapshot/role transition/migrations in a private Coolify job.

DB_ADMIN_URL and generated role passwords belong only to this maintenance job.
Run inspect, backup, transition, migrate and verify separately. The coordinator
stops the old indexer before backup/transition and injects only the new runtime
DSN into the serving app after verification. No credential is printed.
"""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
from urllib.parse import quote, unquote, urlsplit, urlunsplit

TABLES = set("""_sqlx_migrations assets candle_gap_fill_1315 candle_gap_fill_1315_done community_token_events community_tokens defillama_daily_assets defillama_daily_fees defillama_daily_stats economic_token_marks global_liquidity_snapshots global_stats_24h hook_events hub_price_references hub_prices indexer_failed_blocks indexer_schema_version indexer_state limit_order_cancellations limit_order_fills limit_order_placements liquidity_events liquidity_quality_repair_evidence ohlcv_candles oracle_prices oracle_state pair_liquidity_usd pair_reserves pair_volume_24h pair_volume_30d pairs protocol_daily_fees protocol_daily_liquidity protocol_daily_volume protocol_fee_events protocol_fee_stats_by_source protocol_fee_stats_by_token protocol_hourly_fees protocol_hourly_liquidity protocol_hourly_volume protocol_monthly_fees protocol_monthly_liquidity protocol_monthly_volume resting_limit_orders swap_events token_volume_stats trader_positions traders ustc_prices venus_vfdusd_rates""".split())
TABLES.add("candles")  # 20260310000002 renames the original ohlcv_candles table.
# SQLx SHA-384 checksums for the exact fab88 integrated migration artifacts.
EXPECTED_CHECKSUMS = {20260826120000: '4caddc23fdd7f1b0380c8de87889ca331fdc96243ea02c8b2604c978d575df0e6f4d6ce3082ef4b3ab0c6edb0dbe45d8', 20260815000000: '962f0479e145c2345416166f917555a25cdf024901334cdaa199816a9be0ba2b99ffeb32b4b22dcdb8379ad72df107bc', 20261004120000: 'ded6e1ff5ac953f48ab2178749a2476faa1ad76fbc8a65f71d3bd9830876f156083586e31b5f1e21a0e3ab837a088c53', 20260314000001: '76f6d370fd6f5356cbb80f9622a1897c56cbf4fc15fb586ea67a5197c07a6fd69990fc5bc94b5b9498783ec8610066f0', 20260916120001: 'db437e32af48f0f09bd621180381def81894d84b64343e5167629dfecb55d2d2d7a244961b390784cacee26554559290', 20261005064000: '13ec36be9238629896a0940eb84c537b40407389ae0a0713a7e328b7ea6ac8316149a245fa2bb1e1b56599e382ddbe71', 20260506140000: 'c666684d13a7ec145d0160ca6bd8d53028b35f4474117e73023335ebab513bb9ca1cbc22fc226d1e6707ac6dc4cc89b1', 20260826150000: 'f305a7a18d7863c9faa46ba8675e997681b65e8a872644fdd65b6c119f99237ecb8ec11197d564a31acbc8be19ca1b34', 20260825140000: '4726f3d9b02613ae34900ddada08f337c301974a9087c53e0ab3092f4e930a3e622ac61d88287d1411d6554096c79545', 20260829120000: '870d410a4b6ba4fc1dc645d8e03456c2af7767fb23964a786e64b0e25ed2de93474105d2719c4acfd39ff2b06ee45670', 20260707000000: 'a679b00438fd14974c041d726a235eaeea55d26d70c619c6ed08bb5f8486e102c217a522988700cdbbcaeef9a7513186', 20260821120000: 'd5ac7d2251868c57078dddc1ddc3b7917949a7b23c4d769fd6b5db57021a0dd2e52b88e6907a51ab3433124fe456aa38', 20260922120000: '3684357c79de7229a144ba0fa35a2101d549d0e723e1c2b3088da3677047a3d66967419558eb87e93a116056bf1c03de', 20260605000000: '7a30efa8c25256c0221a35b4693ee0667b12a8c561f2a752c4e5956cf8316c1896fd2a79adcaa880bd6f9e4c4accbb45', 20260820100000: '3bd016422d85b4cc23be2df81836d074d75748fccfa22e65a7d6c875fec575f227ed04ffaa8dc20f49473738a198eeb3', 20261004120100: '0e7f1b7f107a6d115afc672c491a37c23285f8f4a5ba973f0ec54ff7e303b70178a837a5e81cbf685cc21b80ad271dd1', 20260823130000: 'c6b0bf6a6df492a4a99db176c20aa50a91a4e9f8457659dd15106b399c04a01c3e0c5c091018fd38a48396d035117810', 20260310000002: '4cd06794ec28e5561da947419b6aed3e1f5e3f58ce7a66717b4701806137f646f4d6492f075f747dd4d1b93619337222', 20260825150000: 'f410d6394ac95e53e6c6d259e156fa45aee7d9c7953450944d4064c3c4671b28b7ae967d5e9f4acd812bcadfc129e0dc', 20260605010100: 'a912f1b170383adda40db4fdb0786ffbcf5ad9e459ea1c8927959d5dace5e190d1b799259415d2634fe9ff41520b5db2', 20260531143000: '45d4f98cb79b3b0e923fcc615b988b6dac8747f911f56dd3a016ae09f13ec13e810928762736285fe92d00fbc37c7b99', 20260921120000: 'e696b2665e76b48792ac05bac0aee761f8b4e3dbbd3d563ceb5c469ca20e0dfea1487a3bf057542dca270cedae933bdc', 20260827160000: 'a83d535a1430ace06a6d6ae865ccf0dde1427ce9e1155c5e6c3580fb16c3dde26992d5c2baf907137fef3301455bf8ed', 20260811000000: '3d65c12414ba361052403e77345bd2ab4f1a06ff717f5004dde60f1679244934e64d4943125e8eaa7c862f91d1ce070b', 20260326120000: 'f82a263b9814ad615c6098d3f87e88b88520c5df15faa1cc70adaa9aad974d61c1d3b010342a2e3dbaa15bd2c7cfb9d4', 20260324000001: 'ac7bb8200053c24f8528c7340f27f3739a0b1d3d1f66c686682fd3c9b26c08f24e40eb5b8a840079b1269b7d4d362d7a', 20260509160000: '1d64308b5710d6e886bed63d7717cfe974cf1cc6eead78272d54e58b2fac8b52d2c0b5c5edda414c98d3266f8224a04e', 20260826180000: 'c186e983d94ce046ca78f8c970286e1e393c0a6d34be27d9191d6951e9bce1fa017957bc7f69660463e062bc2ca47060', 20260827120000: '529c0ea911c03e4d9c53cc5b58dd64f32db08ee163de2e661f3db3ab8cc5c9212ba5f0cb84ac9e9d2ab84087af3a5fbe', 20260513120000: '690be186111767d2617d457c8df25e54d93d7b4efebe91b7f60876594a16b827c3fa83c368484a79453e223f5c52b475', 20260827180000: '94007941eba78d3ccf1af7b10e5aba6d28e3a8fe8a851ee6343bda703d603b8f64418cb15edadf12515e0ee86749f4e6', 20260921120001: 'd0c32693a02035e6b9aa43e355561e386e173b73b807d188ccd56b4acc4ca4af88633ff94dac20b215a0309653818a33', 20260824120000: '5ad08676874bd7203b7380f62ceb7670ce7fa0a48cd933ecd913e766a2fe0684195042cff2c4650483c1fedfdc837358', 20260820120000: '131720e6e6b00338dcc9a4384e9e9b4857fdecbf3e3b2097ad844ce4d33d4d2e8fe959d0cb2a2521bfe65d0ed0b3424d', 20260818140000: '37c461f9da2ebbe26fb07a465353542fc47109f1618266cd12ca62f8856f0aea38d519c21ffd7847374e4a859dfeca6c', 20260818180000: '0a2c8a4fac32fe74fd9074d2058a6e05e81ad65ff436d15c006cb997b7b0aa4e55c1181eb961baf4b1887a4c17640a03', 20260911120000: 'd20ef2b39fde5d1df0e5f3bc4964bdd5deceed832314c73505fe95125009b2c71b997d440cd1c832c1fd27273c5a294c', 20260817000000: 'f2847e410c9556b6121a55e7eab0debb37a7fbed27878297c0ebf283fccd4d9fef1b0ba0656250570cdb4b03418851f6', 20260531120000: '7a9efd9a4de6930c0fa084fd8198fbedd8ad10a836e7a66c24469e160760761c3fed14ece706217ece8bbf7349fa7810', 20260916120000: '1cb0a643b16b1c4331f1f296611b7b5a722b8b239878d2488bbfc3f4aa4c49e392132b6db83c4dcf5e1b27b032bb4fda', 20260820156800: '7d53d9a3d7a77477446e5fc2f5032cf5eecd34a406f4bde81497c29b71f9f48050b1c5c6c9e07549876a1a7d7eacbdc8', 20260818120000: '58581efe9bde7c7b5cf979706e0e253cc9439ac9a34e344cabfe470a90d2262df367405309b13a45c2ca6daa27c610d9', 20261005120001: '52252104b4b6000cf4968f0c78e46c83505a0dca37c36b4aa9ae4563eef30bcccf1817d7ec399e1a8ebb14102e76d0f7', 20260609120000: '62cf214fea56532ed5e90774768b1b9cd9dc5f70da42ee1df881796dc57bfd96771c8027c67b2d226872a494f3233a2f', 20260326000001: 'd73e73f71e90918d98e8492366dc7570987a2763eecd204c4d925febed5b3bd8bd22867908d52d4216ebc2641c4b63b9', 20260310000003: 'f10202084d767f551b0dba30629a665a2010ce9c710d3cb8d3d5342740d975ec82141801a705d7cfc96c984e40f2b553', 20260921130000: '37c50484bc596f6b3fc4c1b29a56ce3dce9cceec498f95058b015e24d0f86391cf575b6e511b940d1b560d887f041404', 20260820200000: '1faa3fba22a5dc80278c497495e70b10cbcdae96c5780ae7130d5b832fbfae6b558cd4d400f88104f4c62b15dd44ab75', 20260604120000: '5886933065265f371ffa1743fe0f27d3d1cebc5aee7e5a747f09acb9e9e76e9048be146ec33ce9c007d13c5da37d4edd', 20260310000001: '648f1f34945e7e67f8a56a729df49a494896cb85856a2389947fa7117898d3a7f508d4dab2eb737b9f65c1951fba2809', 20260825160000: '25cfc4afc1514bf8de8cc055be6238210a8d752cf1250a644f740618f7234f71c6e836834791bf7c02cc875505c1e61a', 20260318000001: '900c3efeb75f71a2245867055ae98540386e259a6dc2a271b2f5af52bb0a8e1f820823130d1507e4435a82ed1f20c395', 20260826140000: '5f8e80ef3c73c4b15f87e18893d8bf3e724bd3306882dc5437b0c27162afe4263fa8026af1f2b76bca039aa64d4aba7a', 20260817120000: '130d8a4db809276954869122e83832a09316cd9b0f002538eefa126f959737db193276bb600d615ed270a1f27f80c785', 20260609140000: '63d23a5e6886fbec92dac2a72916145f63dcddd5852b75c685a8bc2cb39bea6a1818cb43177e7131a211c3e108206d06', 20260604120100: '67898e1103f27a2b655476b938db8d94b2bf4fb2541bb4b56ce40a88e450e11d6afd5014fc3d048408994c71e70c06ec', 20260605010000: '57bbe3fa945fc8aed604f501ecd77a6e44fc07bf76f7de5116be2fb2fd9ac91bff09bdb65f1c8863cc877535a421b4d3', 20261006133000: '9638f0b6f693e6a14dd97822564be42a8fd20fb8437ff3c9648ef2a0cd1c1fab40a5dbb8157d3874b9e3cc82b35938d7', 20260827140000: 'a91d2e5982b3d201c9ea6b07b5372df070a1c872d095cd12a1ffdc6e4f27b18831a7fdd7906c06b046cdee6df3da9f58', 20261005120000: 'aed9d10f04ac06a203898574d824d6915d3d9a4605c600351daae00c45e6bf08e9c5ab1d233bac869958c93492da9bf1', 20260605120000: 'd53f14bf555cf5287e894dc5bb9a704c903b2f0f34d4b378e3fe2d0e0d39c0b2e857c71c4af5ab967501bf2953df5f9c'}


def identifier(value):
    if not re.fullmatch(r"[a-z_][a-z_0-9]{0,62}", value):
        raise RuntimeError("Invalid scoped SQL identifier")
    return '"' + value + '"'


def literal(value):
    return "'" + value.replace("'", "''") + "'"


class Job:
    def __init__(self, directory, binary):
        os.umask(0o077)
        self.directory = directory
        directory.mkdir(parents=True, exist_ok=True)
        directory.chmod(0o700)
        self.binary = binary
        self.admin = urlsplit(os.environ["DB_ADMIN_URL"])
        if self.admin.scheme not in ("postgres", "postgresql") or not self.admin.hostname or not self.admin.username or not self.admin.password or self.admin.query or self.admin.fragment:
            raise RuntimeError("Maintenance administrator DSN shape rejected")
        self.database = os.environ.get("OPS_APPLICATION_DB")
        self.runtime = os.environ.get("INDEXER_RUNTIME_ROLE", "dex_indexer_runtime_20261006")
        self.migrator = os.environ.get("INDEXER_MIGRATOR_ROLE", "dex_indexer_migrator_20261006")
        identifier(self.runtime); identifier(self.migrator)
        if len({self.runtime, self.migrator, unquote(self.admin.username)}) != 3:
            raise RuntimeError("Administrator, migrator and runtime identities must differ")

    def connection(self, database=None, role=None, password=None):
        env = os.environ.copy()
        # subprocess errors and CLI arguments never include a DSN/password.
        env.update(PGHOST=self.admin.hostname, PGPORT=str(self.admin.port or 5432),
                   PGDATABASE=database or self.database or unquote(self.admin.path.lstrip("/")),
                   PGUSER=role or unquote(self.admin.username),
                   PGPASSWORD=password or unquote(self.admin.password),
                   PGAPPNAME="dex-maintenance-20261006", PGCONNECT_TIMEOUT="10")
        env.pop("DATABASE_URL", None)
        return env

    def sql(self, query, database=None, role=None, password=None):
        proc = subprocess.run(["psql", "-X", "-q", "-A", "-t", "-v", "ON_ERROR_STOP=1"],
                              input=query, text=True, capture_output=True,
                              env=self.connection(database, role, password), timeout=300)
        if proc.returncode:
            # SQL errors can contain a role password when a statement is echoed.
            raise RuntimeError("Private maintenance SQL failed; no server error body published")
        return proc.stdout.strip()

    def json_sql(self, query, **kwargs):
        return json.loads(self.sql(query, **kwargs))

    def write(self, name, data):
        target = self.directory / name
        target.write_text(json.dumps(data, indent=2) + "\n")
        target.chmod(0o600)

    def inspect(self):
        names = self.json_sql("SELECT coalesce(json_agg(datname ORDER BY datname),'[]') FROM pg_database WHERE datallowconn AND NOT datistemplate;")
        matches = []
        for name in names:
            signature = self.sql("SELECT to_regclass('public._sqlx_migrations') IS NOT NULL AND to_regclass('public.pairs') IS NOT NULL AND to_regclass('public.swap_events') IS NOT NULL AND to_regclass('public.protocol_fee_events') IS NOT NULL;", database=name)
            if signature == "t":
                matches.append(name)
        if len(matches) != 1 or (self.database and self.database != matches[0]):
            raise RuntimeError("Unique indexer database identity was not established")
        self.database = matches[0]
        identifier(self.database)
        schemas = self.json_sql("SELECT coalesce(json_agg(nspname ORDER BY nspname),'[]') FROM pg_namespace WHERE nspname <> 'information_schema' AND left(nspname,3) <> 'pg_';")
        if schemas != ["public"]:
            raise RuntimeError("Unexpected application schema; scoped ownership transition refused")
        objects = self.json_sql("SELECT coalesce(json_agg(json_build_object('name',c.relname,'kind',c.relkind,'owner',pg_get_userbyid(c.relowner),'acl',c.relacl) ORDER BY c.relname),'[]') FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname='public' AND c.relkind IN ('r','p','v','m','f','S');")
        if any(o["kind"] != "S" and (o["name"] not in TABLES or o["kind"] not in ("r", "p")) for o in objects):
            raise RuntimeError("Public objects exceed the exact indexer table scope")
        orphan_sequences = self.sql("SELECT count(*) FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname='public' AND c.relkind='S' AND NOT EXISTS(SELECT 1 FROM pg_depend d JOIN pg_class t ON t.oid=d.refobjid WHERE d.objid=c.oid AND d.deptype IN ('a','i') AND t.relname IN (" + ",".join(literal(t) for t in sorted(TABLES)) + "));")
        if orphan_sequences != "0" or self.sql("SELECT count(*) FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace WHERE n.nspname='public';") != "0":
            raise RuntimeError("Unscoped function or independent sequence requires reviewed ownership scope")
        migrations = self.json_sql("SELECT coalesce(json_agg(json_build_object('version',version,'checksum',encode(checksum,'hex'),'success',success) ORDER BY version),'[]') FROM public._sqlx_migrations;")
        if any(not m["success"] for m in migrations):
            raise RuntimeError("Existing migration ledger contains a failed migration")
        mismatched = [m["version"] for m in migrations if EXPECTED_CHECKSUMS.get(m["version"]) != m["checksum"]]
        if mismatched:
            self.write("checksum-rejection.json", {"mismatched_versions": mismatched})
            raise RuntimeError("Stored migration checksums differ from the exact release; ledger repair is forbidden")
        result = {"observed_at_utc": datetime.now(timezone.utc).isoformat(), "database": self.database,
                  "schemas": schemas, "objects": objects, "migrations": migrations}
        self.write("inspection.json", result)
        return result

    def assert_stopped(self):
        if os.environ.get("OPS_OLD_INDEXER_STOPPED") != "1":
            raise RuntimeError("Coordinator must stop the old indexer before snapshot/transition")
        sessions = self.sql("SELECT count(*) FROM pg_stat_activity WHERE datname=current_database() AND pid<>pg_backend_pid() AND backend_type='client backend';")
        if sessions != "0":
            raise RuntimeError("Other application database sessions remain; snapshot/transition deferred")

    def backup(self):
        inspection = self.inspect()
        self.assert_stopped()
        target = self.directory / "pretransition.dump"
        if target.exists():
            raise RuntimeError("An existing backup is immutable; reuse its receipt or choose a new operation directory")
        with target.open("xb") as stream:
            proc = subprocess.run(["pg_dump", "--format=custom", "--no-password"],
                                  env=self.connection(), stdout=stream, stderr=subprocess.PIPE, timeout=900)
        if proc.returncode:
            target.unlink(missing_ok=True)
            raise RuntimeError("Private database backup failed")
        listing = subprocess.run(["pg_restore", "--list", str(target)], capture_output=True, timeout=60)
        if listing.returncode or target.stat().st_size < 1024:
            raise RuntimeError("Database backup structural validation failed")
        (self.directory / "pretransition.toc").write_bytes(listing.stdout)
        roles = self.json_sql("SELECT coalesce(json_agg(json_build_object('name',rolname,'superuser',rolsuper,'createdb',rolcreatedb,'createrole',rolcreaterole,'login',rolcanlogin,'inherit',rolinherit,'replication',rolreplication,'bypassrls',rolbypassrls) ORDER BY rolname),'[]') FROM pg_roles WHERE left(rolname,3)<>'pg_';")
        acl = self.json_sql("SELECT json_build_object('database',datname,'owner',pg_get_userbyid(datdba),'acl',datacl) FROM pg_database WHERE datname=current_database();")
        schema_acl = self.json_sql("SELECT json_build_object('name',nspname,'owner',pg_get_userbyid(nspowner),'acl',nspacl) FROM pg_namespace WHERE nspname='public';")
        self.write("pretransition-acl.json", {"database": acl, "schema": schema_acl, "roles": roles, "inventory": inspection})
        receipt = {"database": self.database, "bytes": target.stat().st_size,
                   "sha256": hashlib.file_digest(target.open("rb"), "sha256").hexdigest(),
                   "observed_at_utc": datetime.now(timezone.utc).isoformat(), "structurally_valid": True}
        self.write("backup-receipt.json", receipt)
        return receipt

    def passwords(self):
        runtime = os.environ["DB_RUNTIME_PASSWORD"]
        migrator = os.environ["DB_MIGRATOR_PASSWORD"]
        if min(len(runtime), len(migrator)) < 24 or len({runtime, migrator, unquote(self.admin.password)}) != 3:
            raise RuntimeError("Scoped role credentials must be distinct generated values")
        return runtime, migrator

    def transition(self):
        inventory = self.inspect()
        self.assert_stopped()
        receipt = json.loads((self.directory / "backup-receipt.json").read_text())
        dump = self.directory / "pretransition.dump"
        if receipt["database"] != self.database or hashlib.file_digest(dump.open("rb"), "sha256").hexdigest() != receipt["sha256"]:
            raise RuntimeError("Immutable backup identity/integrity check failed")
        runtime_password, migrator_password = self.passwords()
        statements = ["BEGIN", "SET LOCAL lock_timeout='10s'", "SET LOCAL statement_timeout='60s'"]
        for role, password in [(self.runtime, runtime_password), (self.migrator, migrator_password)]:
            exists = self.sql("SELECT count(*) FROM pg_roles WHERE rolname=" + literal(role)) == "1"
            if exists:
                safe = self.sql("SELECT NOT(rolsuper OR rolcreatedb OR rolcreaterole OR rolreplication OR rolbypassrls OR rolinherit) AND NOT EXISTS(SELECT 1 FROM pg_auth_members WHERE member=r.oid) FROM pg_roles r WHERE rolname=" + literal(role))
                if safe != "t" or self.sql("SELECT current_user", role=role, password=password) != role:
                    raise RuntimeError("Existing scoped role does not match this operation; no password rotation attempted")
            else:
                statements.append("CREATE ROLE " + identifier(role) + " LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS NOINHERIT PASSWORD " + literal(password))
        runtime, migrator, database = map(identifier, (self.runtime, self.migrator, self.database))
        statements += [f"ALTER DATABASE {database} OWNER TO {migrator}",
                       f"REVOKE ALL ON DATABASE {database} FROM PUBLIC, {runtime}",
                       f"GRANT CONNECT ON DATABASE {database} TO {runtime}",
                       f"ALTER SCHEMA public OWNER TO {migrator}", "REVOKE ALL ON SCHEMA public FROM PUBLIC"]
        for obj in inventory["objects"]:
            keyword = "SEQUENCE" if obj["kind"] == "S" else "TABLE"
            statements.append(f"ALTER {keyword} public.{identifier(obj['name'])} OWNER TO {migrator}")
        # Public CONNECT on any unrelated database would bypass the isolation boundary.
        unrelated = self.json_sql("SELECT coalesce(json_agg(datname),'[]') FROM pg_database WHERE datallowconn AND NOT datistemplate AND datname<>current_database();")
        if unrelated:
            raise RuntimeError("Unrelated databases present; no cross-application ACL changes attempted")
        statements.append("COMMIT")
        self.sql(";\n".join(statements) + ";")
        self.write("transition-receipt.json", {"database": self.database, "runtime": self.runtime,
                                             "migrator": self.migrator, "objects_transferred": len(inventory["objects"])})
        return {"transition": "PASS", "objects_transferred": len(inventory["objects"])}

    def role_url(self, role, password):
        authority = quote(role, safe="") + ":" + quote(password, safe="") + "@" + self.admin.hostname + ":" + str(self.admin.port or 5432)
        return urlunsplit(("postgresql", authority, "/" + self.database, "", ""))

    def migrate(self):
        self.inspect()
        self.assert_stopped()
        if not (self.directory / "transition-receipt.json").exists():
            raise RuntimeError("Scoped ownership receipt absent")
        _, password = self.passwords()
        env = {"PATH": os.environ.get("PATH", "/usr/bin:/bin"),
               "DATABASE_URL": self.role_url(self.migrator, password), "INDEXER_RUNTIME_ROLE": self.runtime}
        with (self.directory / "migration-private.log").open("wb") as log:
            result = subprocess.run([self.binary, "migrate"], env=env, stdout=log, stderr=subprocess.STDOUT, timeout=900)
        if result.returncode:
            raise RuntimeError("Scoped migration failed; private log retained, old indexer remains stopped")
        self.write("migration-receipt.json", {"status": "PASS", "binary_sha256": hashlib.file_digest(Path(self.binary).open("rb"), "sha256").hexdigest()})
        return {"migrate": "PASS"}

    def verify(self):
        self.inspect()
        password, _ = self.passwords()
        safe = self.sql("SELECT NOT(rolsuper OR rolcreatedb OR rolcreaterole OR rolreplication OR rolbypassrls OR rolinherit) AND NOT EXISTS(SELECT 1 FROM pg_auth_members WHERE member=r.oid) AND NOT has_database_privilege(current_user,current_database(),'CREATE') AND NOT has_schema_privilege(current_user,'public','CREATE') AND NOT has_table_privilege(current_user,'public._sqlx_migrations','SELECT,INSERT,UPDATE,DELETE,TRUNCATE,REFERENCES,TRIGGER') AND NOT has_table_privilege(current_user,'public.indexer_schema_version','INSERT,UPDATE,DELETE,TRUNCATE,REFERENCES,TRIGGER') FROM pg_roles r WHERE rolname=current_user;", role=self.runtime, password=password)
        if safe != "t":
            raise RuntimeError("Actual runtime ACL verification failed")
        marker = self.sql("SELECT version FROM public.indexer_schema_version WHERE singleton=true", role=self.runtime, password=password)
        inventory = json.loads((self.directory / "pretransition-acl.json").read_text())["inventory"]
        before = {m["version"]: m for m in inventory["migrations"]}
        after = self.json_sql("SELECT json_agg(json_build_object('version',version,'checksum',encode(checksum,'hex'),'success',success) ORDER BY version) FROM public._sqlx_migrations;")
        if not set(before).issubset({m["version"] for m in after}) or any(m != before[m["version"]] for m in after if m["version"] in before):
            raise RuntimeError("Original migration history was altered")
        # Authentication plus zero-row DML establishes permission without changing rows.
        self.sql("BEGIN; SELECT count(*) FROM public.pairs; UPDATE public.indexer_state SET value=value WHERE false; ROLLBACK;", role=self.runtime, password=password)
        for statement in ["SELECT * FROM public._sqlx_migrations LIMIT 0",
                          "UPDATE public.indexer_schema_version SET version=version WHERE false",
                          "CREATE TABLE public.dex_runtime_denial_probe(id integer)",
                          "CREATE ROLE dex_runtime_denial_probe_20261006 NOLOGIN"]:
            proc = subprocess.run(["psql", "-X", "-A", "-t", "-v", "ON_ERROR_STOP=1"],
                                  input="\\set VERBOSITY sqlstate\nBEGIN;\n" + statement + ";\nROLLBACK;\n",
                                  text=True, capture_output=True,
                                  env=self.connection(role=self.runtime, password=password), timeout=30)
            if proc.returncode == 0 or "42501" not in proc.stderr:
                raise RuntimeError("Expected real runtime permission denial was not established")
        evidence = {"runtime_acl": "PASS", "migration_history_preserved": True,
                    "schema_version": marker, "migrations": len(after),
                    "actual_permission_denials": 4}
        self.write("verification.json", evidence)
        return evidence

    def history(self):
        self.inspect()
        queries = {
            "repair_counts": "SELECT coalesce(json_agg(x),'[]') FROM (SELECT source_table,count(*) AS retained_originals FROM liquidity_quality_repair_evidence WHERE repair_key='issue-1380-public-hours' GROUP BY source_table ORDER BY source_table) x",
            "original_hourly": "SELECT coalesce(json_agg(x),'[]') FROM (SELECT source_key,original_row->>'liquidity_usd' AS original_value,original_row->>'priced_pair_count' AS original_priced_pair_count FROM liquidity_quality_repair_evidence WHERE repair_key='issue-1380-public-hours' AND source_table='protocol_hourly_liquidity' ORDER BY source_key LIMIT 2) x",
            "sources": "SELECT coalesce(json_agg(x),'[]') FROM (SELECT sampled_at,total_liquidity_usd::text AS total_liquidity_usd,priced_pair_count,quality,as_of,generation,reasons FROM global_liquidity_snapshots WHERE (sampled_at >= '2026-10-02 05:00Z' AND sampled_at < '2026-10-02 06:00Z') OR (sampled_at >= '2026-10-04 02:00Z' AND sampled_at < '2026-10-04 03:00Z') ORDER BY sampled_at LIMIT 80) x",
            "hourly": "SELECT coalesce(json_agg(x),'[]') FROM (SELECT utc_hour,liquidity_usd::text AS liquidity_usd,priced_pair_count,quality,as_of,coverage_identity,generation,reasons FROM protocol_hourly_liquidity WHERE utc_hour IN ('2026-10-02 05:00Z','2026-10-04 02:00Z') ORDER BY utc_hour) x",
            "daily": "SELECT coalesce(json_agg(x),'[]') FROM (SELECT utc_day,liquidity_usd::text AS liquidity_usd,quality,as_of,coverage_identity,generation,reasons FROM protocol_daily_liquidity WHERE utc_day IN ('2026-10-02','2026-10-04') ORDER BY utc_day) x",
            "monthly": "SELECT coalesce(json_agg(x),'[]') FROM (SELECT utc_month,liquidity_usd::text AS liquidity_usd,quality,as_of,coverage_identity,generation,reasons FROM protocol_monthly_liquidity WHERE utc_month='2026-10-01') x",
            "baselines": "SELECT json_build_object('quality',liquidity_quality,'total',total_liquidity_usd::text,'as_of',liquidity_as_of,'generation',liquidity_generation,'identity',liquidity_coverage_identity,'reasons',liquidity_reasons,'delta_24h',liquidity_change_24h_pct::text,'delta_30d',liquidity_change_30d_pct::text,'baseline_24h',total_liquidity_usd_24h_ago::text,'baseline_30d',total_liquidity_usd_30d_ago::text) FROM global_stats_24h"
        }
        evidence = {name: self.json_sql("BEGIN READ ONLY; SET LOCAL statement_timeout='15s'; " + query + "; ROLLBACK;") for name, query in queries.items()}
        evidence["observed_at_utc"] = datetime.now(timezone.utc).isoformat()
        evidence["exact_amount_rejection_census"] = {
            "status": "UNAVAILABLE",
            "reason": "Parser rejection diagnostics are bounded runtime logs, not persistent database counters"
        }
        # Local QA fixtures may lack the actual production history; mark this
        # explicitly rather than manufacturing retained originals or passed repair.
        if len(evidence["hourly"]) == 2:
            if len(evidence["original_hourly"]) != 2 or any(row["quality"] != "unknown" or row["liquidity_usd"] is not None for row in evidence["hourly"]):
                raise RuntimeError("Documented hourly repairs/original evidence were not established")
            evidence["documented_hourly_repair"] = "PASS"
        else:
            evidence["documented_hourly_repair"] = "UNAVAILABLE"
        self.write("history.json", evidence)
        return evidence

    def apply(self):
        # The immutable backup was created by a prior action while the old
        # indexer was stopped. Every stage still checks actual database state.
        result = {}
        for action in ("transition", "migrate", "verify", "history"):
            self.write("apply-progress.json", {"completed": list(result), "next": action,
                                             "rollback_snapshot": "pretransition.dump",
                                             "old_indexer_must_remain_stopped_on_failure": True})
            result[action] = getattr(self, action)()
        result["status"] = "PASS" if result["history"]["documented_hourly_repair"] == "PASS" else "PASS_WITH_SKIPS"
        self.write("apply-receipt.json", result)
        return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("inspect", "backup", "transition", "migrate", "verify", "history", "apply"))
    parser.add_argument("--directory", type=Path, default=Path("/var/lib/dex-maintenance/release-20261006"))
    parser.add_argument("--binary", default="/usr/local/bin/cl8y-dex-indexer")
    args = parser.parse_args()
    run_id = os.environ.get("OPS_RUN_ID")
    if run_id is not None and not re.fullmatch(r"[a-f0-9]{16,64}", run_id):
        parser.error("OPS_RUN_ID must be a hexadecimal caller nonce")
    job = Job(args.directory, args.binary)
    try:
        result = getattr(job, args.action)()
        if args.action == "inspect":
            result = {"database": result["database"], "objects": len(result["objects"]), "migrations": len(result["migrations"]), "schemas": result["schemas"]}
        print(json.dumps({"action": args.action, "result": result, "run_id": os.environ.get("OPS_RUN_ID"), "observed_at_utc": datetime.now(timezone.utc).isoformat()}))
    except RuntimeError as exc:
        # Every RuntimeError in this module uses a fixed credential-free message.
        print(json.dumps({"action": args.action, "status": "FAIL", "error": str(exc), "run_id": os.environ.get("OPS_RUN_ID")}))
        raise SystemExit(1)
    except Exception:
        # Avoid all raw exceptions: they may include injected environment data.
        print(json.dumps({"action": args.action, "status": "FAIL", "error": "Private job failed; inspect retained protected receipts/logs", "run_id": os.environ.get("OPS_RUN_ID")}))
        raise SystemExit(1)


if __name__ == "__main__":
    main()
