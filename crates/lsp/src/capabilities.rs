use std::cell::OnceCell;

use lsp_types::{CodeActionCapabilityResolveSupport, CodeActionKind, PositionEncodingKind};

const CELL: OnceCell<lsp_types::ClientCapabilities> = OnceCell::new();

pub(crate) fn client_capabilities() -> lsp_types::ClientCapabilities {
    CELL.get_or_init(|| {
        lsp_types::ClientCapabilities {
            workspace: Some(lsp_types::WorkspaceClientCapabilities {
                configuration: Some(true),
                did_change_configuration: Some(lsp_types::DynamicRegistrationClientCapabilities {
                    dynamic_registration: Some(false),
                }),
                workspace_folders: Some(true),
                apply_edit: Some(true),
                symbol: Some(lsp_types::WorkspaceSymbolClientCapabilities {
                    dynamic_registration: Some(false),
                    ..Default::default()
                }),
                execute_command: Some(lsp_types::DynamicRegistrationClientCapabilities {
                    dynamic_registration: Some(false),
                }),
                inlay_hint: Some(lsp_types::InlayHintWorkspaceClientCapabilities {
                    refresh_support: Some(false),
                }),
                workspace_edit: Some(lsp_types::WorkspaceEditClientCapabilities {
                    document_changes: Some(true),
                    resource_operations: Some(vec![
                        lsp_types::ResourceOperationKind::Create,
                        lsp_types::ResourceOperationKind::Rename,
                        lsp_types::ResourceOperationKind::Delete,
                    ]),
                    failure_handling: Some(lsp_types::FailureHandlingKind::Abort),
                    normalizes_line_endings: Some(false),
                    change_annotation_support: None,
                }),
                did_change_watched_files: Some(
                    lsp_types::DidChangeWatchedFilesClientCapabilities {
                        dynamic_registration: Some(true),
                        relative_pattern_support: Some(false),
                    },
                ),
                file_operations: Some(lsp_types::WorkspaceFileOperationsClientCapabilities {
                    // will_rename: Some(true),
                    // did_rename: Some(true),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            text_document: Some(lsp_types::TextDocumentClientCapabilities {
                completion: Some(lsp_types::CompletionClientCapabilities {
                    completion_item: Some(lsp_types::CompletionItemCapability {
                        snippet_support: Some(false),
                        resolve_support: Some(lsp_types::CompletionItemCapabilityResolveSupport {
                            properties: vec![
                                String::from("documentation"),
                                String::from("detail"),
                                String::from("additionalTextEdits"),
                            ],
                        }),
                        insert_replace_support: Some(true),
                        deprecated_support: Some(true),
                        tag_support: Some(lsp_types::TagSupport {
                            value_set: vec![lsp_types::CompletionItemTag::DEPRECATED],
                        }),
                        ..Default::default()
                    }),
                    completion_item_kind: Some(lsp_types::CompletionItemKindCapability {
                        ..Default::default()
                    }),
                    context_support: None, // additional context information Some(true)
                    ..Default::default()
                }),
                hover: Some(lsp_types::HoverClientCapabilities {
                    // if not specified, rust-analyzer returns plaintext marked as markdown but
                    // badly formatted.
                    content_format: Some(vec![lsp_types::MarkupKind::Markdown]),
                    ..Default::default()
                }),
                signature_help: None,
                // signature_help: Some(lsp_types::SignatureHelpClientCapabilities {
                //     signature_information: Some(lsp_types::SignatureInformationSettings {
                //         documentation_format: Some(vec![lsp_types::MarkupKind::Markdown]),
                //         parameter_information: Some(lsp_types::ParameterInformationSettings {
                //             label_offset_support: Some(true),
                //         }),
                //         active_parameter_support: Some(true),
                //     }),
                //     ..Default::default()
                // }),
                rename: Some(lsp_types::RenameClientCapabilities {
                    dynamic_registration: Some(false),
                    prepare_support: Some(true),
                    prepare_support_default_behavior: None,
                    honors_change_annotations: Some(false),
                }),
                formatting: Some(lsp_types::DocumentFormattingClientCapabilities {
                    dynamic_registration: Some(false),
                }),
                code_action: Some(lsp_types::CodeActionClientCapabilities {
                    code_action_literal_support: Some(lsp_types::CodeActionLiteralSupport {
                        code_action_kind: lsp_types::CodeActionKindLiteralSupport {
                            value_set: [
                                CodeActionKind::EMPTY,
                                CodeActionKind::QUICKFIX,
                                CodeActionKind::REFACTOR,
                                CodeActionKind::REFACTOR_EXTRACT,
                                CodeActionKind::REFACTOR_INLINE,
                                CodeActionKind::REFACTOR_REWRITE,
                                CodeActionKind::SOURCE,
                                CodeActionKind::SOURCE_ORGANIZE_IMPORTS,
                            ]
                            .iter()
                            .map(|kind| kind.as_str().to_string())
                            .collect(),
                        },
                    }),
                    is_preferred_support: Some(true),
                    disabled_support: Some(true),
                    data_support: Some(true),
                    resolve_support: Some(CodeActionCapabilityResolveSupport {
                        properties: vec!["edit".to_owned(), "command".to_owned()],
                    }),
                    ..Default::default()
                }),
                publish_diagnostics: None,
                // publish_diagnostics: Some(lsp_types::PublishDiagnosticsClientCapabilities {
                //     version_support: Some(true),
                //     tag_support: Some(lsp_types::TagSupport {
                //         value_set: vec![
                //             lsp_types::DiagnosticTag::UNNECESSARY,
                //             lsp_types::DiagnosticTag::DEPRECATED,
                //         ],
                //     }),
                //     ..Default::default()
                // }),
                inlay_hint: Some(lsp_types::InlayHintClientCapabilities {
                    dynamic_registration: Some(false),
                    resolve_support: None,
                }),
                ..Default::default()
            }),
            window: Some(lsp_types::WindowClientCapabilities {
                work_done_progress: Some(true),
                ..Default::default()
            }),
            general: Some(lsp_types::GeneralClientCapabilities {
                position_encodings: Some(vec![
                    PositionEncodingKind::UTF8,
                    PositionEncodingKind::UTF32,
                    PositionEncodingKind::UTF16,
                ]),
                ..Default::default()
            }),
            ..Default::default()
        }
    })
    .clone()
}
