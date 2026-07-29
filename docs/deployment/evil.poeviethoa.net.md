# Evil Hunter Deployment

The live deployment uses the `evil-prod` Docker network and these container
roles:

- `evil-prod-postgres` with volume `evil-prod-postgres-data`;
- `evil-prod-redis` with volume `evil-prod-redis-data`;
- `evil-prod-server` on the internal `server` alias;
- `evil-prod-web` published only on `127.0.0.1:25174`.

The host Nginx virtual host proxies `evil.poeviethoa.net` to the web container
and passes WebSocket upgrade headers. PostgreSQL and Redis are not exposed
through the public interface. The server uses `SESSION_COOKIE_SECURE=true` and
`WEB_ORIGIN=https://evil.poeviethoa.net`.

After rebuilding either image, recreate only the matching `evil-prod-*`
container and verify:

```sh
curl -fsS https://evil.poeviethoa.net/healthz
curl -fsS https://evil.poeviethoa.net/ready
```

`deploy/smoke-production.sh` checks the HTTPS web shell and server dependency
readiness. Use a browser journey for the secure-cookie/WSS flow; do not disable
the secure cookie or Origin checks in production.

## Host prerequisites

- Docker network `evil-prod` and volumes `evil-prod-postgres-data` and
  `evil-prod-redis-data` must already exist.
- PostgreSQL and Redis stay private on that network under aliases `postgres`
  and `redis`.
- Host Nginx proxies the public hostname to `127.0.0.1:25174`; the checked-in
  virtual host is `deploy/evil.poeviethoa.net.nginx.conf`.
- The wildcard Let's Encrypt certificate must cover `evil.poeviethoa.net`.
- The server must use `WEB_ORIGIN=https://evil.poeviethoa.net` and
  `SESSION_COOKIE_SECURE=true`. Do not expose the database or Redis ports.

The current deployment uses the internal database URL
`postgres://evil_hunter:evil_hunter@postgres:5432/evil_hunter`. Replace that
development-grade password before treating the host as a public production
service; keep the replacement outside the repository.

## Rebuild and recreate

Build both candidates before stopping a running container:

```sh
docker build -f apps/server/Dockerfile -t evil-prod-server:candidate .
docker build -f apps/web/Dockerfile \
  --build-arg VITE_WORLD_WS_URL=wss://evil.poeviethoa.net/ws \
  --build-arg VITE_WORLD_API_URL=https://evil.poeviethoa.net \
  -t evil-prod-web:candidate .
```

Apply forward-only migrations against the private production network:

```sh
docker run --rm --network evil-prod \
  -e DATABASE_URL=postgres://evil_hunter:evil_hunter@postgres:5432/evil_hunter \
  -v "$PWD/infra/db:/migrations:ro" \
  postgres:17-alpine /bin/sh /migrations/run-migrations.sh
```

The original production publication of
`0010_normalized_building_gameplay_content` records checksum `53fe119c...`,
while the canonical generated artifact retained in Git has checksum
`218ffa02...`. The migration runner accepts only this exact legacy/canonical
pair, preserves the recorded production checksum, and still fails closed for
every other mismatch. Static catalogs under `core_game` remain a separate,
reproducible initialization step.

Keep the previous images as rollback targets, then recreate only the app
containers. The persistent database and Redis containers are not replaced:

```sh
docker tag evil-prod-server:latest evil-prod-server:rollback
docker tag evil-prod-web:latest evil-prod-web:rollback
docker tag evil-prod-server:candidate evil-prod-server:latest
docker tag evil-prod-web:candidate evil-prod-web:latest

docker rm -f evil-prod-server
docker run -d --name evil-prod-server --restart unless-stopped \
  --network evil-prod --network-alias server \
  -p 127.0.0.1:28082:8080 \
  -e SERVER_HOST=0.0.0.0 -e SERVER_PORT=8080 \
  -e RUST_LOG=evil_hunter_server=info,tower_http=info -e LOG_FORMAT=pretty \
  -e DATABASE_URL=postgres://evil_hunter:evil_hunter@postgres:5432/evil_hunter \
  -e REDIS_URL=redis://redis:6379/0 \
  -e WEB_ORIGIN=https://evil.poeviethoa.net \
  -e SESSION_COOKIE_SECURE=true -e SESSION_TTL_SECONDS=604800 \
  -e SIMULATION_TICK_RATE=10 -e SIMULATION_SEED=6840227782638526189 \
  -e PLAYER_LEASE_TTL_MS=15000 -e COMMAND_RATE_LIMIT=30 \
  -e COMMAND_RATE_WINDOW_MS=1000 \
  evil-prod-server:latest

curl --fail --retry 20 --retry-delay 1 http://127.0.0.1:28082/ready

docker rm -f evil-prod-web
docker run -d --name evil-prod-web --restart unless-stopped \
  --network evil-prod -p 127.0.0.1:25174:80 evil-prod-web:latest
```

Validate Nginx and the public path after recreation:

```sh
nginx -t
./deploy/smoke-production.sh
```

If an app container fails, retag its `:rollback` image as `:latest`, recreate
that container with the same command, and rerun the smoke script. Database
migrations are forward-only and are not rolled back with the image.
