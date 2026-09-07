# Skenario Pengujian: DataOrm (DATA)

> Skenario pengujian untuk fitur DataOrm — Model, Query Builder, Migrations, Vector.
> Per `qa-design §1–2`, `bdd-gherkin` + `api-contract-test` + `migration-test` + `chaos`.

## Header & Navigation

- [Module Overview](../../modules/data-orm/overview.md)
- [API Specification](../../api/data-orm/api-query-builder.md)

## 1. Positive Cases (Happy Path)

| ID | User Story | Test Case | Pre-condition | Input Data | Expected Result | Priority |
|----|------------|-----------|---------------|------------|-----------------|----------|
| DATA-POS-001 | US-M2-01 | Fluent filter returns expected record | `users` row status active | `where("status","active").first().await` | `Some(User{id:1})` | High |
| DATA-POS-002 | US-M2-04 | serde round-trip preserves relations | user Ada 2 posts eager | `to_string(&users); from_str<Vec<User>>` | `relation_loaded("posts")` true + 2 posts | High |
| DATA-POS-003 | US-M2-02 | chunkBy 10k→20×500 without OOM | 10k users | `chunkBy("id",500)` | 20 batches ×500 | High |
| DATA-POS-004 | US-M2-02 | insertOrIgnoreReturning inserts non-conflicting + returns IDs | some IDs conflict | `insertOrIgnoreReturning(rows)` | non-conflicts inserted, IDs returned | High |
| DATA-POS-005 | US-M2-05 | make:model + migrate creates posts collection | `make:model Post -m` + `migrate` | `SELECT * FROM posts` | table exists | High |
| DATA-POS-006 | US-M2-05 | Factory sequence resets between tests | factory seq 5 in prior test | next `Factory::create(1)` | email for sequence 1 | High |
| DATA-POS-007 | US-M2-06 | Vector nearest-neighbor returns k ordered | 100 products embeddings + query near 10 | `whereVectorSimilarTo(..., limit:10)` | 10 rows ordered by cosine | High |
| DATA-POS-008 | US-M2-01 | Transactional lock serializes | two concurrent `select_for_update` on Ada | concurrent `transaction()` with lock | second waits until first commits | High |

## 2. Negative Cases (Validation & Errors)

| ID | User Story | Test Case | Pre-condition | Input Data | Expected Result | Priority |
|----|------------|-----------|---------------|------------|-----------------|----------|
| DATA-NEG-001 | US-M2-03 | Upsert EmptyUniqueBy before round-trip | `upsert(rows, unique_by:[], update:["name"])` | `upsert` exec | `UpsertError::EmptyUniqueBy`, no rows written | High |
| DATA-NEG-002 | US-M2-02 | whereBinary on non-binary column | Postgres non-binary col | `whereBinary` | `IncompatibleColumn` | Medium |
| DATA-NEG-003 | US-M2-04 | Empty relation round-trips as empty list | user Ada no posts eager | serialize then deserialize | posts `[]` not `None`, `relation_loaded` true | Medium |
| DATA-NEG-004 | US-M2-05 | Query missing collection → MissingTable | no migration for posts | `Post::query().list()` | `MissingTable{ name:"posts"}` | Medium |
| DATA-NEG-005 | US-M2-06 | Extension missing guidance | pg vector not installed | migration with `vector(1536)` | `ExtensionMissing` with hint | High |
| DATA-NEG-006 | US-M2-06 | Dimension mismatch outline | column 1536 vs query 768 (and reverse) | `whereVectorSimilarTo` dim mismatch | `VectorDimensionMismatch{ expected:1536, actual:768 }` (see `fixtures/vector-dim.json`) | High |
| DATA-NEG-007 | US-M2-02 | MySQL delete join compilation (MySQL driver only) | MySQL driver + join+order | `DELETE … JOIN` | generated SQL contains `DELETE users FROM users JOIN orders` | Medium |

## 3. Monkey Testing (Chaos & Stability)

| ID | Focus | Test Case | Pre-condition | Expected Result |
|----|-------|-----------|---------------|-----------------|
| DATA-MNK-001 | Concurrency | two `select_for_update` contenders | lock via transaction | no dirty read, second blocks |
| DATA-MNK-002 | OOM guard | 10k chunkBy contenders + pool sized via config | `chunkBy("id", 500)` | all 20 batches visited, no OOM (`qa-design N-Boundary`) |
| DATA-MNK-003 | Migration round-trip idempotence | `up → present → down → absent → up →present` | `database/migrations/*` | round-trip proof (see below §5) |
| DATA-MNK-004 | `dropVectorIndex` mid-search | `dropVectorIndex` while `whereVectorSimilarTo` active | HNSW → seq scan | still returns results (TC-M6-25 adjacency) |
| DATA-MNK-005 | `toSql` snapshot drift | `toSql`/`toRawSql` per driver | `cargo insta` snapshot | approved snapshot per `cargo insta review` |
| DATA-MNK-006 | Pool lost mid-tx | kill PG container mid-transaction | `QueryError::PoolClosed` | retryable, not dangling tx |
| DATA-MNK-007 | Property: arbitrary relations round-trip | 0..20 relations | `proptest` TC-PROP-01 | serde round-trip invariant holds |

## 4. Security Testing

| ID | Role | Test Case | Action | Expected Result |
|----|------|-----------|--------|-----------------|
| DATA-SEC-001 | Attacker | `where("status", "' OR 1=1")` injection probe | payload in `where` bound param | prepared-statement bound, not string-concatenated |
| DATA-SEC-002 | Developer | `vector` null until `toEmbeddings` fills | insert without embedding | column nullable, not constraint panic |
| DATA-SEC-003 | Auditor | `InsertOrIgnoreReturning` with null element | `Vec<Option<User>>` containing `None` | serde round-trip handles `None` entries without panic |
| DATA-SEC-004 | Auditor | `collection-serialization` depth | `User→Post→User` cycle depth 3 | depth limit prevents infinite serialize |

## 5. Migration Testing (`migration-test` rules)

- **Round-trip:** `up → assert table/column/index exists → down → assert absent → up → assert exists`.
- **Idempotence:** `migrate` twice consecutively no-op (row count / schema hash unchanged).
- **Bulk:** `migrate:fresh --seed` → seeded row PKs retrievable via `get`.
- **Irreversible:** declared `Irreversible { name }` → `down` emits `MigrationError::Irreversible{name}`.
- Fixtures live in `crates/rustavel-orm/tests/migration_roundtrip.rs` (QA design asserts harness shape).

