# DataLens Rust Monolith

Серверная часть DataLens на Rust — **два бинарника**:

* **`datalens-monolith`** (~18 MB) — United Storage, data-api, ui-api/charts-engine,
  control-api и meta-manager в одном процессе;
* **[`datalens-auth-rs`](auth/)** (~3.5 MB) — отдельный сервис авторизации (login, JWT,
  users) вместо `datalens-auth:0.27.0`.

Вместе они заменяют пять Python/Node-сервисов и отдают байт-в-байт те же чартовые
payload'ы. Браузерный UI — официальный контейнер `datalens-ui` на `:8091`/`:8092`.

* **База эквивалентности:** DataLens **v2.9.0** (`ui 0.3831.0`, `us 1.39.0`,
  backend `0.2457.0`, auth `0.27.0`, meta-manager `0.52.0`).
* **Parity:** 57/57 chart payload'ов из реального дашборда совпадают с Python-стеком
  (см. [`docs/PARITY.md`](docs/PARITY.md)).
* **Память:** холодный процесс ~26 MB, после трафика ~90 MB, пик при 64 клиентах ~190 MB
  (см. [`docs/MEMORY.md`](docs/MEMORY.md)).
* **Батч и стриминг:** `POST /api/run/batch` отдаёт пачку одним JSON, а
  `POST /api/run/batch/stream` — NDJSON-строку на чарт, как только он готов. Ручки,
  разница в семантике и замеры — [`docs/PERFORMANCE.md`](docs/PERFORMANCE.md).
* **V8/Node на серверной стороне нет.** Два небольших бинарника без интерпретаторов и JIT.
* нужен **один бинарник без отдельного auth-процесса**? Есть форк:
  [datalens-rust-monolith-embedded-auth](https://github.com/ivan-vibe/datalens-rust-monolith-embedded-auth).

## Что внутри

```
src/                     монолит: US + data-api + ui-api + control-api + meta-manager
vendor/data-api/crates/  Rust-порт data-api (dl_core, dl_query, dl_api, dl_us, ...)
vendor/us-rust           Rust-порт United Storage
auth/                    datalens-auth-rs (login, JWT, users) — отдельный сервис
deploy/                  docker-compose кластер, Dockerfile'ы, seed БД, bootstrap секретов
docs/                   архитектура, parity, память, auth
scripts/                 сборка и вспомогательные утилиты
```

## Готовые бинарники

В git они не лежат — соберите локально (`cargo build --release`) или возьмите из
[GitHub Releases](../../releases) этой сборки: `macOS arm64`, `macOS x86_64`,
`Linux x86_64` (musl, статический), `Windows x86_64`. Каждый — один бинарник
(~18 MB), серверная часть целиком.

## Быстрый старт (Docker)

Требования: Docker + docker compose v2, ~4 GB RAM, свободные порты `8091`, `8099`.

```bash
git clone <repo> datalens-rust-monolith && cd datalens-rust-monolith
docker compose up -d --build        # первый раз ~10 мин: сборка образов + демо-данные
```

Или та же команда через обёртку: `./deploy/cluster.sh up`.

Первый запуск:

1. официальный `ghcr.io/datalens-tech/datalens-postgres:16-1.3.0` создаёт базы
   `pg-us-db`, `pg-auth-db`, `pg-meta-manager-db`, `pg-demo-db` и демо-схему;
2. `monolith` накатывает US-миграции и поднимается на `:8099`;
3. one-shot сервис `seed` заливает **официальные демо-данные** из `/init/demo-data` того же
   образа Postgres — в репозитории нет ни одного дампа и ни одной пользовательской строки;
4. поднимаются `auth` и `datalens-ui:0.3831.0`.

| URL | Что |
|---|---|
| `http://localhost:8091` | UI |
| `http://localhost:8099/ping` | монолит (API) |

Логин: **admin / admin** (демо-пользователь официального образа). Смените пароль.

Остановить / снести:

```bash
docker compose down        # остановить, volume с БД сохранить
docker compose down -v     # остановить и удалить volume (демо-данные зальются заново)
```

### Ключи по умолчанию

В `deploy/docker-compose.yml` стоят **публичные демо-ключи открытого DataLens** (те же, что
в официальном `datalens-tech/datalens`): пара PS256, `US_MASTER_TOKEN=us-master-token`,
`AUTH_MASTER_TOKEN=auth-master-token`, fernet-ключ для паролей подключений. Они нужны, чтобы
демо работало «из коробки», и **не являются секретами** — они опубликованы upstream.

Для чего-то большего демо переопределите их своим окружением или запустите
`./deploy/bootstrap.sh` (сгенерирует ключи в `deploy/secrets/compose.secrets.yml`,
`cluster.sh` подхватит его поверх дефолтов).

## Сборка без Docker

MSRV: Rust **1.85+**.

```bash
cargo build --release          # → target/release/datalens-monolith
cd auth && cargo build --release   # → auth/target/release/datalens-auth-rs
```

Запуск на хосте требует внешнего PostgreSQL с базами `pg-us-db`, `pg-auth-db`,
`pg-meta-manager-db`, `pg-demo-db` (демо-данные — из официального образа Postgres). Переменные окружения —
как в `deploy/docker-compose.yml` (сервисы `monolith`/`auth`), с поправкой на адреса БД;
`POSTGRES_DSN_LIST` указывает на `pg-us-db`, `META_MANAGER_PG_DSN` — на `pg-meta-manager-db`.

## Конфигурация

Основные переменные окружения монолита (полный список — в `deploy/docker-compose.yml`):

| Переменная | Значение по умолчанию | Смысл |
|---|---|---|
| `POSTGRES_DSN_LIST` | — | DSN United Storage (`pg-us-db`) |
| `META_MANAGER_PG_DSN` | — | DSN durable job store (`pg-meta-manager-db`) |
| `AUTH__TYPE` / `AUTH__JWT_ALGORITHM` | `NATIVE` / `PS256` | режим проверки JWT |
| `AUTH__JWT_KEY` | — | публичный ключ auth (PEM) |
| `US_MASTER_TOKEN`, `AUTH_MASTER_TOKEN` | — | сервисные токены |
| `PUBLIC_PORT` | `8099` | публичный порт монолита |
| `DL_CONNECTION_HOST_MAP` | — | подмена хоста в connection-настройках датасетов (`postgres=postgres:5432`) |
| `DL_COMPRESSION` / `DL_COMPRESS_MIN_CHUNK` | `zstd` / `4096` | сжатие ответов `off/gzip/zstd` |
| `DL_ZSTD_LEVEL` / `DL_ZSTD_STREAM_LEVEL` | `3` / `1` | уровень zstd для обычных ответов и для NDJSON-потока |
| `DL_BATCH_CONCURRENCY` | `8` | чартов одного батча одновременно (`0` — без лимита, максимум 64) |
| `DL_STREAM_ITEM_TIMEOUT_SECS` | `60` | дедлайн на чарт в стриме (`0` — выключить) |
| `DL_RUN_TIMEOUT_SECS` | `120` | внешний дедлайн `/api/run*` (`0` — выключить); у стрима ограничивает только время до заголовков |
| `DL_PG_STATEMENT_TIMEOUT_MS` / `DL_PG_POOL_WAIT_SECS` / `DL_PG_CONNECT_TIMEOUT_SECS` | `180000` / `30` / `10` | дедлайн запроса на стороне PostgreSQL, ожидание слота пула, таймаут соединения |
| `DL_US_LOCAL_TIMEOUT_SECS` | `60` | дедлайн in-process чтения US |
| `DL_DB_POOL_SIZE` | `20` | пул соединений на источник |
| `TOKIO_WORKER_THREADS` | `7` | воркеры tokio (на хосте с 8+ ядрами) |
| `US_ENTRY_CACHE_TTL_SECS`, `US_ENTRY_CACHE_MAX` | `60` / `1024` | кэш US-записей |
| `CHART_EXTRA_CACHE_MAX`, `PARSED_SHARED_CACHE_MAX`, `TABLE_REQUEST_CACHE_MAX` | `512/512/256` | лимиты render-кэшей |
| `MIMALLOC_PURGE_DELAY` | — | `0` включает агрессивный возврат памяти ОС |

## API (основное)

| Метод и путь | Что |
|---|---|
| `POST /api/run` | рендер чарта (тот же контракт, что у ui-api) |
| `POST /api/run/batch` | несколько чартов одним запросом |
| `POST /api/run/batch/stream` | те же чарты потоком: NDJSON, строка на чарт по мере готовности |
| `GET/POST /api/v1/*` | control-api: датасеты, соединения, валидаторы, импорт/экспорт |
| `POST /api/data/v2/*` | data-api: result/preview/pivot/values |
| `GET /private/*`, `/api/v1/entries` | United Storage (чтение записей) |
| `GET /meta/*` | meta-manager: коллекции, воркбуки, импорт/экспорт |
| `GET /ping` | liveness |
| `GET /debug/cache`, `/debug/stages` | admin-only диагностика (кэши, стадии) |

## Тесты

```bash
cargo test --offline        # 143 unit-теста монолита + vendor-крейтов
cd auth && cargo test --offline
```

Дифференциальные тесты с настоящим DataLens (parity) требуют референсный Python-кластер и
HAR-файл с запросами; как это запускается — [`docs/PARITY.md`](docs/PARITY.md).

## Аудит и известные ограничения

Результаты аудита кода (безопасность, хрупкие места, мёртвый код) — [docs/AUDIT.md](docs/AUDIT.md).

### Безопасность

Аутентификация включена по умолчанию (`AUTH_ENABLED` не задан = включена). В
`MONOLITH`-профиле процесс откажется стартовать, если auth включён, а `AUTH__TYPE`
не `NATIVE`, либо пусты `US_MASTER_TOKEN`/`AUTH__JWT_KEY`. Debug-эндпоинты закрыты токеном
admin; без in-process auth состояния они отвечают `503`. Явный локальный режим без auth —
только `AUTH_ENABLED=0`.

Права записи проверяет US: control-api валидирует JWT и пишет от имени пользователя
(публичные `/v1/entries`), поэтому viewer не может менять датасеты/соединения. Сервисные
`/private/*` маршруты US защищены мастер-токеном: `deploy/bootstrap.sh` генерирует
случайный токен, UI-контейнер использует его для своих service-to-service вызовов
(tenant-настройки). Auth-rs дополнительно пускает свои `/private/v1/*` только с loopback.
Для интернет-экспозиции поставьте TLS-reverse-proxy, смените `admin/admin` и
сгенерируйте собственные мастер-токены (`deploy/bootstrap.sh --force`).

### Ограничения

* UI-патч для Gravity-чарта не входит в репозиторий: в консоли браузера может появляться
  `It seems you are trying to get non-existing category value` (баг самого UI, на данные и
  серверную часть не влияет; чинится подменой одного чанка в `datalens-ui`).
* Parity покрывает реальные чарты демо-дашборда (line/bar/area/pie/table/pivot/geo) и
  контрольные API-маршруты; редкие фичи (extracts, некоторые RLS-сценарии, отчёты) могут
  отличаться — приветствуются issues с HAR-запросом.
* Кластер одноузловой: Postgres, auth и монолит — по одному инстансу. Горизонтальное
  масштабирование монолита возможно за балансировщиком (stateless, состояние — в Postgres).
* Демо-пароль `admin/admin` — только для локального запуска.

## Лицензия

Apache License 2.0 — см. [LICENSE](LICENSE). Проект является производной работой от
[DataLens](https://github.com/datalens-tech/datalens) (Apache-2.0); см. [NOTICE](NOTICE).

**Дисклеймер:** это неофициальный проект, не связанный с DataLens, его мейнтейнерами или
Яндексом и не одобренный ими. DataLens и связанные логотипы — товарные знаки
соответствующих правообладателей.

