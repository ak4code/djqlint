# DjQlint — Django Query Linter

🌍 [English](README.md) · **Русский**

Быстрый статический анализатор промышленного уровня (на Rust), который находит
неэффективные запросы Django ORM и антипаттерны базы данных в Python-коде —
**без импорта и запуска вашего проекта**. Разбор исходников выполняется через
[`tree-sitter`](https://tree-sitter.github.io/), а вывод в формате **SARIF**
интегрируется с GitHub Advanced Security, SonarQube и GitLab.

```text
  ! `if pending:` evaluates the full QuerySet just to test for rows
    ,-[app/views.py:16:8]
 15 |     pending = Book.objects.filter(status="pending")
 16 |     if pending:
    :        ^^^|^^^
    :           `-- truthiness test loads every row
 17 |         notify_admins()
    `----
  help: Use `if pending.exists():` — it issues a cheap `SELECT 1 … LIMIT 1`
        instead of fetching and caching the whole result set.
```

## Правила

| Код    | Имя                   | Что находит |
|--------|-----------------------|-------------|
| `N001` | `n-plus-one`          | Итерация по QuerySet с обращением к связанному объекту на каждой строке (`for b in qs: b.author.name`) без `select_related`/`prefetch_related`. |
| `P001` | `in-memory-counting`  | `len(queryset)` / `list(queryset)` — затягивает все строки в память Python. Нужно `.count()` / `.exists()`. |
| `P002` | `redundant-truthiness`| `if queryset:` вычисляет и кэширует весь результат только ради проверки наличия строк. Нужно `.exists()`. |

## Установка и запуск

```bash
cargo build --release

./target/release/djqlint path/to/project        # красивый вывод в консоль
./target/release/djqlint -f json   .             # машиночитаемый JSON
./target/release/djqlint -f sarif -o out.sarif . # SARIF для CI
./target/release/djqlint --list-rules
```

Код возврата: `1` — найдены проблемы (переопределяется через `--exit-zero`),
`2` — внутренняя ошибка, `0` — чисто. Готово для гейта в CI.

## Конфигурация — `.djqlint.toml`

```toml
[rules]
disabled = ["P002"]          # отключение правил по коду

[files]
exclude = ["/migrations/", "/tests/"]
respect_gitignore = true     # по умолчанию
```

## Подавление срабатываний inline-комментариями

```python
total = len(qs)                       # noqa: djqlint-P001   (одно правило)
total = len(qs)                       # noqa                 (все правила на строке)
total = len(qs)                       # djqlint: disable     (все правила djqlint)
total = len(qs)                       # djqlint: disable=P001,N001
```

## Архитектура

```
src/
├── main.rs              # тонкий бинарник: разбор аргументов -> djqlint::run
├── lib.rs              # run(): оркестрация config, обход, анализ, отчёт
├── cli.rs              # описание CLI (clap, derive)
├── config.rs           # .djqlint.toml (serde + toml)
├── diagnostic.rs       # Diagnostic / Severity / FileReport (общие типы)
├── engine/
│   ├── mod.rs          # контекст FileUnit + параллельный драйвер (rayon)
│   ├── walker.rs       # обход ФС с учётом .gitignore (крейт ignore)
│   ├── parser.rs       # парсинг tree-sitter + компиляция запросов
│   ├── queryset.rs     # эвристическая модель потока данных QuerySet (трекинг переменных)
│   ├── suppress.rs     # разбор директив noqa / djqlint:disable
│   └── util.rs         # помощники по AST (текст узла, pre-order обход курсором)
├── rules/
│   ├── mod.rs          # `trait Rule`, реестр, общие помощники запросов
│   ├── queries/*.scm   # S-выражения tree-sitter (include_str!)
│   ├── n001_nplus1.rs
│   ├── p001_in_memory_count.rs
│   └── p002_redundant_truthiness.rs
└── reporters/
    ├── console.rs      # графический отчёт miette (с подсветкой исходника)
    ├── json.rs         # стабильный JSON
    └── sarif.rs        # SARIF 2.1.0
```

### Как устроен анализ

1. **Обход** (`engine::walker`) — `ignore::WalkBuilder` собирает `*.py`,
   учитывая `.gitignore` и исключения из конфига.
2. **Распараллеливание** (`engine::analyze_files`) — `rayon` обрабатывает каждый
   файл параллельно. Для каждого файла создаётся свой парсер `tree-sitter`
   (парсеры `Send`, но не `Sync`).
3. **Парсинг + модель** — `tree-sitter` восстанавливается после ошибок, поэтому
   некорректный Python никогда не приводит к панике; затем строится
   консервативная модель `Bindings`, отслеживающая, какие локальные переменные
   хранят QuerySet (`qs = User.objects.filter(...)`), до достижения неподвижной
   точки, чтобы захватить и транзитивные присваивания.
4. **Запуск правил** — каждое `Rule` владеет заранее скомпилированным запросом
   `tree-sitter` (неизменяемым и `Sync`, поэтому он разделяется между потоками).
   Правила сопоставляют грубые синтаксические формы через запрос, а затем
   уточняют результат с помощью модели потока данных.
5. **Подавление** — срабатывания на строках с директивой
   `noqa`/`djqlint:disable` отбрасываются.
6. **Отчёт** — консоль (miette), JSON или SARIF.

### Как добавить правило

1. Положите S-выражение в `src/rules/queries/<код>.scm`.
2. Реализуйте `trait Rule` (скомпилируйте запрос в `new()` через
   `include_str!`, сопоставьте в `check()`) — канонический пример в
   `p001_in_memory_count.rs`.
3. Зарегистрируйте его в `rules::build_rules`.

## CI

* **GitHub Actions** — `.github/workflows/ci.yml` собирает, линтит, тестирует,
  затем загружает SARIF в code scanning.
* **GitLab CI** — `.gitlab-ci.yml` отдаёт SARIF как SAST-отчёт.

## Разработка

```bash
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test --all
cargo run -- examples/views.py        # демо-фикстура, задействующая все правила
```

## Лицензия

MIT.
