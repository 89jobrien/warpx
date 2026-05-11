use super::*;

fn create_db(path: &std::path::Path) {
    let conn = rusqlite::Connection::open(path).unwrap();
    conn.execute_batch(
        "CREATE TABLE checkpoints (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            project TEXT NOT NULL,
            cwd TEXT NOT NULL,
            generated TEXT NOT NULL,
            recommendation TEXT,
            json_path TEXT NOT NULL,
            created_at TEXT DEFAULT (datetime('now'))
        );",
    )
    .unwrap();
}

fn insert(conn: &rusqlite::Connection, project: &str, rec: Option<&str>, created: &str) {
    conn.execute(
        "INSERT INTO checkpoints (project, cwd, generated, recommendation, json_path, created_at) \
         VALUES (?1, '/tmp', '2026-05-11', ?2, '/tmp/h.json', ?3)",
        rusqlite::params![project, rec, created],
    )
    .unwrap();
}

#[test]
fn load_from_populated_db() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("handup.db");
    create_db(&db);
    let conn = rusqlite::Connection::open(&db).unwrap();
    insert(&conn, "alpha", Some("fix alpha"), "2026-05-10 01:00:00");
    insert(&conn, "beta", Some("fix beta"), "2026-05-11 02:00:00");
    insert(&conn, "alpha", Some("update alpha"), "2026-05-11 03:00:00");
    drop(conn);

    let results = HandupModel::query_checkpoints(&db).unwrap();
    assert_eq!(results.len(), 3);
    // Most recent first
    assert_eq!(results[0].project, "alpha");
    assert_eq!(results[0].recommendation.as_deref(), Some("update alpha"));
    assert_eq!(results[1].project, "beta");
    assert_eq!(results[2].project, "alpha");
}

#[test]
fn load_empty_db() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("handup.db");
    create_db(&db);

    let results = HandupModel::query_checkpoints(&db).unwrap();
    assert!(results.is_empty());
}

#[test]
fn load_missing_db() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("nonexistent.db");
    let err = HandupModel::query_checkpoints(&db).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn load_bad_schema() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("handup.db");
    let conn = rusqlite::Connection::open(&db).unwrap();
    conn.execute_batch("CREATE TABLE wrong_table (x TEXT);")
        .unwrap();
    drop(conn);

    let err = HandupModel::query_checkpoints(&db).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn null_recommendation() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("handup.db");
    create_db(&db);
    let conn = rusqlite::Connection::open(&db).unwrap();
    insert(&conn, "proj", None, "2026-05-11 01:00:00");
    drop(conn);

    let results = HandupModel::query_checkpoints(&db).unwrap();
    assert_eq!(results.len(), 1);
    assert!(results[0].recommendation.is_none());
}
