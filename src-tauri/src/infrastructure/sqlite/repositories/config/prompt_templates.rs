//! Prompt template CRUD + seed for the config store.

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};

use super::helpers::parse_datetime;
use super::ConfigRepositoryError;
use crate::core::config::PromptTemplate;

/// Seeded-in-code definition of a builtin template. Kept compact so the
/// reset path can rebuild the row without the database.
struct BuiltinTemplate {
    slug: &'static str,
    title: &'static str,
    description: &'static str,
    icon: &'static str,
    category: &'static str,
    content: &'static str,
    sort_order: i64,
}

const BUILTIN_TEMPLATES: &[BuiltinTemplate] = &[
    BuiltinTemplate {
        slug: "generate",
        title: "Generate",
        description: "Generate code from a description",
        icon: "code",
        category: "code",
        content: "Generate code based on the following description. Output only the code unless asked otherwise.\n\n{args}",
        sort_order: 0,
    },
    BuiltinTemplate {
        slug: "explain",
        title: "Explain",
        description: "Explain selected code",
        icon: "book",
        category: "code",
        content: "Explain the following code in detail: what it does, how it works, and any non-obvious behavior.\n\n{args}",
        sort_order: 1,
    },
    BuiltinTemplate {
        slug: "refactor",
        title: "Refactor",
        description: "Refactor selected code",
        icon: "refactor",
        category: "code",
        content: "Refactor the following code while preserving behavior. Improve readability, maintainability, and idiomatic style. Briefly note what you changed and why.\n\n{args}",
        sort_order: 2,
    },
    BuiltinTemplate {
        slug: "optimize",
        title: "Optimize",
        description: "Optimize selected code",
        icon: "bolt",
        category: "code",
        content: "Optimize the following code for performance. Identify the bottleneck, apply the fix, and explain the trade-offs.\n\n{args}",
        sort_order: 3,
    },
    BuiltinTemplate {
        slug: "test",
        title: "Test",
        description: "Generate tests for selected code",
        icon: "beaker",
        category: "code",
        content: "Write comprehensive tests for the following code. Cover the happy path and realistic edge cases. Use the project's existing test framework and style.\n\n{args}",
        sort_order: 4,
    },
    BuiltinTemplate {
        slug: "document",
        title: "Document",
        description: "Write documentation for selected code",
        icon: "document",
        category: "code",
        content: "Write clear documentation for the following code: purpose, parameters, return values, usage examples, and any caveats.\n\n{args}",
        sort_order: 5,
    },
];

fn builtin_by_slug(slug: &str) -> Option<&'static BuiltinTemplate> {
    BUILTIN_TEMPLATES.iter().find(|b| b.slug == slug)
}

fn row_to_template(row: &rusqlite::Row<'_>) -> rusqlite::Result<PromptTemplate> {
    let created_at_str: String = row.get(10)?;
    let updated_at_str: String = row.get(11)?;
    Ok(PromptTemplate {
        id: row.get(0)?,
        slug: row.get(1)?,
        title: row.get(2)?,
        description: row.get(3)?,
        icon: row.get(4)?,
        category: row.get(5)?,
        content: row.get(6)?,
        is_builtin: row.get::<_, i64>(7)? != 0,
        is_enabled: row.get::<_, i64>(8)? != 0,
        sort_order: row.get(9)?,
        created_at: parse_datetime(&created_at_str),
        updated_at: parse_datetime(&updated_at_str),
    })
}

const SELECT_COLUMNS: &str = "id, slug, title, description, icon, category, content, is_builtin, is_enabled, sort_order, created_at, updated_at";

/// Lists every template, ordered by `sort_order` then `slug`.
pub fn list_prompt_templates(
    conn: &Connection,
) -> Result<Vec<PromptTemplate>, ConfigRepositoryError> {
    let sql =
        format!("SELECT {SELECT_COLUMNS} FROM prompt_templates ORDER BY sort_order ASC, slug ASC");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt
        .query_map([], row_to_template)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Loads a template by slug.
pub fn get_prompt_template_by_slug(
    conn: &Connection,
    slug: &str,
) -> Result<Option<PromptTemplate>, ConfigRepositoryError> {
    let sql = format!("SELECT {SELECT_COLUMNS} FROM prompt_templates WHERE slug = ?1");
    let mut stmt = conn.prepare(&sql)?;
    let template = stmt.query_row(params![slug], row_to_template).optional()?;
    Ok(template)
}

/// Loads a template by id.
pub fn get_prompt_template_by_id(
    conn: &Connection,
    id: &str,
) -> Result<Option<PromptTemplate>, ConfigRepositoryError> {
    let sql = format!("SELECT {SELECT_COLUMNS} FROM prompt_templates WHERE id = ?1");
    let mut stmt = conn.prepare(&sql)?;
    let template = stmt.query_row(params![id], row_to_template).optional()?;
    Ok(template)
}

/// Inserts or updates a template. Returns the persisted row.
pub fn upsert_prompt_template(
    conn: &Connection,
    template: &PromptTemplate,
) -> Result<PromptTemplate, ConfigRepositoryError> {
    conn.execute(
        "INSERT INTO prompt_templates (id, slug, title, description, icon, category, content, is_builtin, is_enabled, sort_order, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
         ON CONFLICT(id) DO UPDATE SET
            slug = excluded.slug,
            title = excluded.title,
            description = excluded.description,
            icon = excluded.icon,
            category = excluded.category,
            content = excluded.content,
            is_enabled = excluded.is_enabled,
            sort_order = excluded.sort_order,
            updated_at = excluded.updated_at",
        params![
            &template.id,
            &template.slug,
            &template.title,
            &template.description,
            &template.icon,
            &template.category,
            &template.content,
            i64::from(template.is_builtin),
            i64::from(template.is_enabled),
            template.sort_order,
            &template.created_at.to_rfc3339(),
            &template.updated_at.to_rfc3339(),
        ],
    )?;
    get_prompt_template_by_id(conn, &template.id)?
        .ok_or_else(|| ConfigRepositoryError::NotFound("PromptTemplate", template.id.clone()))
}

/// Deletes a custom template by id. Builtins are rejected so the user can't
/// remove them from the palette; the UI routes them through `reset` instead.
pub fn delete_prompt_template(conn: &Connection, id: &str) -> Result<(), ConfigRepositoryError> {
    let rows = conn.execute(
        "DELETE FROM prompt_templates WHERE id = ?1 AND is_builtin = 0",
        params![id],
    )?;
    if rows == 0 {
        return Err(ConfigRepositoryError::NotFound(
            "PromptTemplate",
            id.to_string(),
        ));
    }
    Ok(())
}

/// Resets a builtin template's row to its seeded default. Returns the
/// refreshed row. Fails when `slug` is not a known builtin.
pub fn reset_prompt_template(
    conn: &Connection,
    slug: &str,
) -> Result<PromptTemplate, ConfigRepositoryError> {
    let builtin = builtin_by_slug(slug)
        .ok_or_else(|| ConfigRepositoryError::NotFound("PromptTemplate", slug.to_string()))?;
    conn.execute(
        "DELETE FROM prompt_templates WHERE slug = ?1",
        params![slug],
    )?;
    insert_builtin(conn, builtin)?;
    get_prompt_template_by_slug(conn, slug)?
        .ok_or_else(|| ConfigRepositoryError::NotFound("PromptTemplate", slug.to_string()))
}

fn insert_builtin(conn: &Connection, b: &BuiltinTemplate) -> Result<(), ConfigRepositoryError> {
    let now = Utc::now().to_rfc3339();
    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT OR IGNORE INTO prompt_templates (id, slug, title, description, icon, category, content, is_builtin, is_enabled, sort_order, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, 1, ?8, ?9, ?9)",
        params![
            &id,
            b.slug,
            b.title,
            b.description,
            Some(b.icon),
            b.category,
            b.content,
            b.sort_order,
            now,
        ],
    )?;
    Ok(())
}

/// Seeds every builtin template. Idempotent: `INSERT OR IGNORE` on the unique
/// `slug` column leaves any user-edited row untouched.
pub fn seed_builtin_prompt_templates(conn: &Connection) -> Result<(), ConfigRepositoryError> {
    for b in BUILTIN_TEMPLATES {
        insert_builtin(conn, b)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::infrastructure::sqlite::migrations::config_schema::init_schema(&conn).unwrap();
        seed_builtin_prompt_templates(&conn).unwrap();
        conn
    }

    #[test]
    fn seed_inserts_all_builtins() {
        let conn = make_conn();
        let rows = list_prompt_templates(&conn).unwrap();
        assert_eq!(rows.len(), BUILTIN_TEMPLATES.len());
        assert!(rows.iter().all(|r| r.is_builtin));
    }

    #[test]
    fn seed_is_idempotent_and_preserves_edits() {
        let conn = make_conn();

        // Edit the refactor builtin in place.
        let mut refactor = get_prompt_template_by_slug(&conn, "refactor")
            .unwrap()
            .expect("refactor template");
        refactor.content = "EDITED:\n\n{args}".to_string();
        upsert_prompt_template(&conn, &refactor).unwrap();

        // Running seed again must not overwrite the edit.
        seed_builtin_prompt_templates(&conn).unwrap();
        let after = get_prompt_template_by_slug(&conn, "refactor")
            .unwrap()
            .unwrap();
        assert_eq!(after.content, "EDITED:\n\n{args}");
    }

    #[test]
    fn delete_rejects_builtins() {
        let conn = make_conn();
        let refactor = get_prompt_template_by_slug(&conn, "refactor")
            .unwrap()
            .unwrap();
        let err = delete_prompt_template(&conn, &refactor.id);
        assert!(err.is_err());
    }

    #[test]
    fn delete_removes_custom_templates() {
        let conn = make_conn();
        let now = Utc::now();
        let custom = PromptTemplate {
            id: uuid::Uuid::new_v4().to_string(),
            slug: "commit-msg".to_string(),
            title: "Commit message".to_string(),
            description: "Write a commit message".to_string(),
            icon: None,
            category: "CUSTOM".to_string(),
            content: "Write a commit message for:\n\n{args}".to_string(),
            is_builtin: false,
            is_enabled: true,
            sort_order: 100,
            created_at: now,
            updated_at: now,
        };
        let saved = upsert_prompt_template(&conn, &custom).unwrap();
        delete_prompt_template(&conn, &saved.id).unwrap();
        assert!(get_prompt_template_by_slug(&conn, "commit-msg")
            .unwrap()
            .is_none());
    }

    #[test]
    fn reset_restores_default_content_verbatim() {
        let conn = make_conn();

        // Mutate refactor.
        let mut refactor = get_prompt_template_by_slug(&conn, "refactor")
            .unwrap()
            .unwrap();
        refactor.content = "EDITED:\n\n{args}".to_string();
        upsert_prompt_template(&conn, &refactor).unwrap();

        // Reset.
        let reset = reset_prompt_template(&conn, "refactor").unwrap();
        let expected = builtin_by_slug("refactor").unwrap().content;
        assert_eq!(reset.content, expected);
    }

    #[test]
    fn reset_of_unknown_slug_errors() {
        let conn = make_conn();
        let err = reset_prompt_template(&conn, "no-such-slug");
        assert!(err.is_err());
    }

    #[test]
    fn upsert_can_insert_and_update() {
        let conn = make_conn();
        let now = Utc::now();
        let mut custom = PromptTemplate {
            id: uuid::Uuid::new_v4().to_string(),
            slug: "note".to_string(),
            title: "Note".to_string(),
            description: "".to_string(),
            icon: None,
            category: "CUSTOM".to_string(),
            content: "v1 {args}".to_string(),
            is_builtin: false,
            is_enabled: true,
            sort_order: 50,
            created_at: now,
            updated_at: now,
        };
        upsert_prompt_template(&conn, &custom).unwrap();
        custom.content = "v2 {args}".to_string();
        custom.updated_at = Utc::now();
        let updated = upsert_prompt_template(&conn, &custom).unwrap();
        assert_eq!(updated.content, "v2 {args}");
    }
}
