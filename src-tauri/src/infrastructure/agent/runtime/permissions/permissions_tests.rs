#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::agent::session::SessionId;
    use crate::core::agent::types::TrustMode;
    use serde_json::json;

    #[test]
    fn test_pending_action_ext_with_preview() {
        let pending = PendingAction::new(
            SessionId("test".to_string()),
            "test_tool".to_string(),
            ActionType::ReadFile {
                path: "test.txt".to_string(),
            },
            "Test action".to_string(),
            json!({}),
        );

        let preview = ActionPreview {
            content_type: "diff".to_string(),
            content: "test diff".to_string(),
            language: Some("rust".to_string()),
        };

        let with_preview = pending.with_preview(preview.clone());
        assert!(with_preview.preview.is_some());
        assert_eq!(with_preview.preview.unwrap().content, "test diff");
    }

    #[test]
    fn test_pending_action_creation() {
        let session_id = SessionId("session-123".to_string());
        let action = PendingAction::new(
            session_id.clone(),
            "fs_write_file".to_string(),
            ActionType::WriteFile {
                path: "/tmp/test.txt".to_string(),
                size_bytes: 100,
            },
            "Write a test file".to_string(),
            json!({"path": "/tmp/test.txt", "content": "hello"}),
        );

        assert_eq!(action.session_id.0, "session-123");
        assert_eq!(action.tool_name, "fs_write_file");
        assert!(action.preview.is_none());
    }

    #[test]
    fn test_action_type_read_file() {
        let action = ActionType::ReadFile {
            path: "/path/to/file.rs".to_string(),
        };

        let serialized = serde_json::to_string(&action).unwrap();
        assert!(serialized.contains("read_file"));

        let deserialized: ActionType = serde_json::from_str(&serialized).unwrap();
        match deserialized {
            ActionType::ReadFile { path } => {
                assert_eq!(path, "/path/to/file.rs");
            }
            _ => panic!("Expected ReadFile"),
        }
    }

    #[test]
    fn test_action_type_write_file() {
        let action = ActionType::WriteFile {
            path: "/path/to/file.rs".to_string(),
            size_bytes: 1024,
        };

        let serialized = serde_json::to_string(&action).unwrap();
        assert!(serialized.contains("write_file"));
        assert!(serialized.contains("1024"));
    }

    #[test]
    fn test_action_type_delete_file() {
        let action = ActionType::DeleteFile {
            path: "/tmp/old_file.txt".to_string(),
        };

        let serialized = serde_json::to_string(&action).unwrap();
        assert!(serialized.contains("delete_file"));
    }

    #[test]
    fn test_action_type_execute_command() {
        let action = ActionType::ExecuteCommand {
            command: "cargo test".to_string(),
        };

        let serialized = serde_json::to_string(&action).unwrap();
        assert!(serialized.contains("execute_command"));
        assert!(serialized.contains("cargo test"));
    }

    #[test]
    fn test_action_type_web_request() {
        let action = ActionType::WebRequest {
            url: "https://example.com".to_string(),
        };

        let serialized = serde_json::to_string(&action).unwrap();
        assert!(serialized.contains("web_request"));
        assert!(serialized.contains("https://example.com"));
    }

    #[test]
    fn test_action_preview_creation() {
        let preview = ActionPreview {
            content_type: "diff".to_string(),
            content: "--- old\n+++ new".to_string(),
            language: Some("typescript".to_string()),
        };

        let serialized = serde_json::to_string(&preview).unwrap();
        assert!(serialized.contains("diff"));
        assert!(serialized.contains("typescript"));
    }

    #[test]
    fn test_action_preview_without_language() {
        let preview = ActionPreview {
            content_type: "text".to_string(),
            content: "Plain text preview".to_string(),
            language: None,
        };

        let serialized = serde_json::to_string(&preview).unwrap();
        assert!(serialized.contains("text"));

        let deserialized: ActionPreview = serde_json::from_str(&serialized).unwrap();
        assert!(deserialized.language.is_none());
    }

    #[test]
    fn test_pending_action_with_multiple_previews() {
        let session_id = SessionId("test".to_string());
        let base = PendingAction::new(
            session_id,
            "test_tool".to_string(),
            ActionType::ReadFile {
                path: "test.txt".to_string(),
            },
            "Test".to_string(),
            json!({}),
        );

        let preview1 = ActionPreview {
            content_type: "diff".to_string(),
            content: "diff content".to_string(),
            language: Some("rust".to_string()),
        };

        let with_preview1 = base.with_preview(preview1);
        assert!(with_preview1.preview.is_some());
        assert_eq!(
            with_preview1.preview.unwrap().language,
            Some("rust".to_string())
        );
    }

    #[test]
    fn test_permission_level_serialization() {
        let levels = vec![
            crate::core::agent::types::PermissionLevel::AutoRun,
            crate::core::agent::types::PermissionLevel::ApprovalRequired,
            crate::core::agent::types::PermissionLevel::Disabled,
        ];

        for level in levels {
            let serialized = serde_json::to_string(&level).unwrap();
            let deserialized: crate::core::agent::types::PermissionLevel =
                serde_json::from_str(&serialized).unwrap();
            assert_eq!(level, deserialized);
        }
    }

    #[test]
    fn test_trust_mode_serialization() {
        let modes = vec![
            TrustMode::Trusted,
            TrustMode::Untrusted,
            TrustMode::Restricted,
        ];

        for mode in modes {
            let serialized = serde_json::to_string(&mode).unwrap();
            let deserialized: TrustMode = serde_json::from_str(&serialized).unwrap();
            assert_eq!(mode, deserialized);
        }
    }

    #[test]
    fn test_permission_level_auto_run() {
        let level = crate::core::agent::types::PermissionLevel::AutoRun;
        let serialized = serde_json::to_string(&level).unwrap();
        assert!(serialized.contains("auto_run"));
    }

    #[test]
    fn test_permission_level_approval_required() {
        let level = crate::core::agent::types::PermissionLevel::ApprovalRequired;
        let serialized = serde_json::to_string(&level).unwrap();
        assert!(serialized.contains("approval_required"));
    }

    #[test]
    fn test_permission_level_disabled() {
        let level = crate::core::agent::types::PermissionLevel::Disabled;
        let serialized = serde_json::to_string(&level).unwrap();
        assert!(serialized.contains("disabled"));
    }

    #[test]
    fn test_pending_action_with_all_fields() {
        let action = PendingAction::new(
            SessionId("session-123".to_string()),
            "fs_write_file".to_string(),
            ActionType::WriteFile {
                path: "/tmp/test.txt".to_string(),
                size_bytes: 100,
            },
            "Write test file".to_string(),
            json!({"path": "/tmp/test.txt", "content": "hello"}),
        );

        assert_eq!(action.session_id.0, "session-123");
        assert_eq!(action.tool_name, "fs_write_file");
        assert!(action.preview.is_none());
    }

    #[test]
    fn test_pending_action_id_format() {
        let action = PendingAction::new(
            SessionId("test".to_string()),
            "test_tool".to_string(),
            ActionType::ReadFile {
                path: "test.txt".to_string(),
            },
            "Test".to_string(),
            json!({}),
        );

        assert!(!action.id.is_empty());
        assert!(action.id.len() > 10);
    }

    #[test]
    fn test_pending_action_serialization() {
        let action = PendingAction::new(
            SessionId("session-456".to_string()),
            "delete_file".to_string(),
            ActionType::DeleteFile {
                path: "/tmp/old.txt".to_string(),
            },
            "Delete old file".to_string(),
            json!({"path": "/tmp/old.txt"}),
        );

        let serialized = serde_json::to_string(&action).unwrap();
        let deserialized: PendingAction = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.session_id.0, "session-456");
        assert_eq!(deserialized.tool_name, "delete_file");
    }

    #[test]
    fn test_pending_action_with_preview_serialization() {
        let action = PendingAction::new(
            SessionId("test".to_string()),
            "write_file".to_string(),
            ActionType::WriteFile {
                path: "/tmp/test.txt".to_string(),
                size_bytes: 50,
            },
            "Write file".to_string(),
            json!({}),
        );

        let preview = ActionPreview {
            content_type: "diff".to_string(),
            content: "--- old\n+++ new".to_string(),
            language: Some("rust".to_string()),
        };

        let with_preview = action.with_preview(preview);

        let serialized = serde_json::to_string(&with_preview).unwrap();
        let deserialized: PendingAction = serde_json::from_str(&serialized).unwrap();

        assert!(deserialized.preview.is_some());
        assert_eq!(deserialized.preview.unwrap().content_type, "diff");
    }

    #[test]
    fn test_pending_action_multiple_previews_replace() {
        let action = PendingAction::new(
            SessionId("test".to_string()),
            "tool".to_string(),
            ActionType::ReadFile {
                path: "file.txt".to_string(),
            },
            "Read file".to_string(),
            json!({}),
        );

        let preview1 = ActionPreview {
            content_type: "text".to_string(),
            content: "First preview".to_string(),
            language: None,
        };

        let preview2 = ActionPreview {
            content_type: "diff".to_string(),
            content: "Second preview".to_string(),
            language: Some("typescript".to_string()),
        };

        let with_first = action.with_preview(preview1);
        let with_second = with_first.with_preview(preview2);

        assert_eq!(with_second.preview.unwrap().content, "Second preview");
    }

    #[test]
    fn test_action_type_read_file_fields() {
        let action = ActionType::ReadFile {
            path: "/home/user/project/src/main.rs".to_string(),
        };

        let serialized = serde_json::to_string(&action).unwrap();
        assert!(serialized.contains("read_file"));
        assert!(serialized.contains("/home/user/project/src/main.rs"));
    }

    #[test]
    fn test_action_type_write_file_fields() {
        let action = ActionType::WriteFile {
            path: "/tmp/output.json".to_string(),
            size_bytes: 2048,
        };

        let serialized = serde_json::to_string(&action).unwrap();
        assert!(serialized.contains("write_file"));
        assert!(serialized.contains("2048"));
    }

    #[test]
    fn test_action_type_delete_file_fields() {
        let action = ActionType::DeleteFile {
            path: "/tmp/unused.tmp".to_string(),
        };

        let serialized = serde_json::to_string(&action).unwrap();
        assert!(serialized.contains("delete_file"));
    }

    #[test]
    fn test_action_type_execute_command_fields() {
        let action = ActionType::ExecuteCommand {
            command: "cargo test --release".to_string(),
        };

        let serialized = serde_json::to_string(&action).unwrap();
        assert!(serialized.contains("execute_command"));
        assert!(serialized.contains("cargo test --release"));
    }

    #[test]
    fn test_action_type_web_request_fields() {
        let action = ActionType::WebRequest {
            url: "https://api.example.com/v1/users".to_string(),
        };

        let serialized = serde_json::to_string(&action).unwrap();
        assert!(serialized.contains("web_request"));
        assert!(serialized.contains("https://api.example.com"));
    }

    #[test]
    fn test_action_type_all_variants_roundtrip() {
        let actions = vec![
            ActionType::ReadFile {
                path: "/path/to/file.rs".to_string(),
            },
            ActionType::WriteFile {
                path: "/path/to/file.rs".to_string(),
                size_bytes: 100,
            },
            ActionType::DeleteFile {
                path: "/path/to/file.rs".to_string(),
            },
            ActionType::ExecuteCommand {
                command: "cargo build".to_string(),
            },
            ActionType::WebRequest {
                url: "https://example.com".to_string(),
            },
        ];

        for action in actions {
            let serialized = serde_json::to_string(&action).unwrap();
            let deserialized: ActionType = serde_json::from_str(&serialized).unwrap();

            match (&action, &deserialized) {
                (ActionType::ReadFile { path: p1 }, ActionType::ReadFile { path: p2 }) => {
                    assert_eq!(p1, p2);
                }
                (
                    ActionType::WriteFile {
                        path: p1,
                        size_bytes: s1,
                    },
                    ActionType::WriteFile {
                        path: p2,
                        size_bytes: s2,
                    },
                ) => {
                    assert_eq!(p1, p2);
                    assert_eq!(s1, s2);
                }
                (ActionType::DeleteFile { path: p1 }, ActionType::DeleteFile { path: p2 }) => {
                    assert_eq!(p1, p2);
                }
                (
                    ActionType::ExecuteCommand { command: c1 },
                    ActionType::ExecuteCommand { command: c2 },
                ) => {
                    assert_eq!(c1, c2);
                }
                (ActionType::WebRequest { url: u1 }, ActionType::WebRequest { url: u2 }) => {
                    assert_eq!(u1, u2);
                }
                _ => panic!("Action types don't match"),
            }
        }
    }

    #[test]
    fn test_runtime_session_state_from_core_processing_maps_to_running() {
        let runtime = runtime_session_state_from_core(
            &crate::core::agent::entities::SessionState::Processing {
                started_at: Utc::now(),
            },
        );

        match runtime {
            crate::core::agent::session::SessionState::Running { .. } => {}
            other => panic!("Expected Running state, got {:?}", other),
        }
    }

    #[test]
    fn test_action_preview_all_fields() {
        let preview = ActionPreview {
            content_type: "diff".to_string(),
            content: "--- a/file.rs\n+++ b/file.rs\n@@ -1,1 +1,1 @@\n-old\n+new".to_string(),
            language: Some("rust".to_string()),
        };

        let serialized = serde_json::to_string(&preview).unwrap();
        assert!(serialized.contains("diff"));
        assert!(serialized.contains("rust"));
    }

    #[test]
    fn test_action_preview_roundtrip() {
        let preview = ActionPreview {
            content_type: "text".to_string(),
            content: "Plain text content".to_string(),
            language: None,
        };

        let serialized = serde_json::to_string(&preview).unwrap();
        let deserialized: ActionPreview = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.content_type, "text");
        assert_eq!(deserialized.content, "Plain text content");
        assert!(deserialized.language.is_none());
    }

    #[test]
    fn test_action_preview_with_language_none() {
        let previews = vec![
            ActionPreview {
                content_type: "text".to_string(),
                content: "content".to_string(),
                language: None,
            },
            ActionPreview {
                content_type: "json".to_string(),
                content: "{}".to_string(),
                language: None,
            },
        ];

        for preview in previews {
            assert!(preview.language.is_none());
        }
    }

    #[test]
    fn test_action_preview_with_various_languages() {
        let languages = vec![
            ("rust", "diff"),
            ("typescript", "diff"),
            ("python", "diff"),
            ("javascript", "diff"),
            ("json", "json"),
            ("html", "html"),
        ];

        for (lang, content_type) in languages {
            let preview = ActionPreview {
                content_type: content_type.to_string(),
                content: format!("{} content", lang),
                language: Some(lang.to_string()),
            };

            assert_eq!(preview.language.unwrap(), lang);
        }
    }

    #[test]
    fn test_pending_action_ext_trait_available() {
        let action = PendingAction::new(
            SessionId("test".to_string()),
            "test_tool".to_string(),
            ActionType::ReadFile {
                path: "test.txt".to_string(),
            },
            "Test".to_string(),
            json!({}),
        );

        let preview = ActionPreview {
            content_type: "text".to_string(),
            content: "Preview".to_string(),
            language: None,
        };

        let _modified = action.with_preview(preview);
    }

    #[test]
    fn test_pending_action_ext_preserves_other_fields() {
        let action = PendingAction::new(
            SessionId("session-xyz".to_string()),
            "custom_tool".to_string(),
            ActionType::ExecuteCommand {
                command: "npm install".to_string(),
            },
            "Install packages".to_string(),
            json!({"command": "npm install"}),
        );

        let original_id = action.id.clone();
        let original_session_id = action.session_id.0.clone();
        let original_tool_name = action.tool_name.clone();

        let preview = ActionPreview {
            content_type: "text".to_string(),
            content: "Will run: npm install".to_string(),
            language: None,
        };

        let modified = action.with_preview(preview);

        assert_eq!(modified.id, original_id);
        assert_eq!(modified.session_id.0, original_session_id);
        assert_eq!(modified.tool_name, original_tool_name);
        assert!(modified.preview.is_some());
    }

    #[test]
    fn test_approval_timeout_constant() {
        assert_eq!(APPROVAL_TIMEOUT_SECS, 300);
    }

    #[test]
    fn test_approval_timeout_reasonable() {
        assert!(APPROVAL_TIMEOUT_SECS >= 60);
        assert!(APPROVAL_TIMEOUT_SECS <= 600);
    }

    #[test]
    fn test_trust_mode_affects_permission_level() {
        let modes = vec![
            TrustMode::Trusted,
            TrustMode::Untrusted,
            TrustMode::Restricted,
        ];

        for mode in modes {
            let serialized = serde_json::to_string(&mode).unwrap();
            let _deserialized: TrustMode = serde_json::from_str(&serialized).unwrap();
        }
    }

    #[test]
    fn test_action_type_edge_cases() {
        let action = ActionType::ReadFile {
            path: "".to_string(),
        };
        let serialized = serde_json::to_string(&action).unwrap();
        let deserialized: ActionType = serde_json::from_str(&serialized).unwrap();
        match deserialized {
            ActionType::ReadFile { path } => assert_eq!(path, ""),
            _ => panic!("Expected ReadFile"),
        }

        let long_path = "/".repeat(100) + "file.txt";
        let action = ActionType::WriteFile {
            path: long_path.clone(),
            size_bytes: 0,
        };
        let serialized = serde_json::to_string(&action).unwrap();
        let deserialized: ActionType = serde_json::from_str(&serialized).unwrap();
        match deserialized {
            ActionType::WriteFile { path, .. } => assert_eq!(path, long_path),
            _ => panic!("Expected WriteFile"),
        }

        let action = ActionType::WriteFile {
            path: "empty.txt".to_string(),
            size_bytes: 0,
        };
        let serialized = serde_json::to_string(&action).unwrap();
        let deserialized: ActionType = serde_json::from_str(&serialized).unwrap();
        match deserialized {
            ActionType::WriteFile { size_bytes, .. } => assert_eq!(size_bytes, 0),
            _ => panic!("Expected WriteFile"),
        }

        let action = ActionType::WriteFile {
            path: "large.bin".to_string(),
            size_bytes: 1_000_000_000,
        };
        let serialized = serde_json::to_string(&action).unwrap();
        let deserialized: ActionType = serde_json::from_str(&serialized).unwrap();
        match deserialized {
            ActionType::WriteFile { size_bytes, .. } => assert_eq!(size_bytes, 1_000_000_000),
            _ => panic!("Expected WriteFile"),
        }
    }

    #[test]
    fn test_action_preview_with_special_characters() {
        let preview = ActionPreview {
            content_type: "diff".to_string(),
            content: "--- old\n+++ new\n@@ -1,1 +1,1 @@\n-Hello, 世界\n+Hello, World\n".to_string(),
            language: Some("rust".to_string()),
        };

        let serialized = serde_json::to_string(&preview).unwrap();
        let deserialized: ActionPreview = serde_json::from_str(&serialized).unwrap();

        assert!(deserialized.content.contains("世界"));
    }

    #[test]
    fn test_pending_action_with_complex_arguments() {
        let complex_args = json!({
            "files": ["file1.rs", "file2.rs", "file3.rs"],
            "options": {
                "recursive": true,
                "force": false
            },
            "filters": [
                {"type": "extension", "value": ".rs"},
                {"type": "size", "operator": ">", "value": 100}
            ]
        });

        let action = PendingAction::new(
            SessionId("test".to_string()),
            "complex_tool".to_string(),
            ActionType::ReadFile {
                path: "test.txt".to_string(),
            },
            "Complex operation".to_string(),
            complex_args,
        );

        assert!(action.arguments["files"].is_array());
        assert_eq!(action.arguments["files"].as_array().unwrap().len(), 3);
    }
}
