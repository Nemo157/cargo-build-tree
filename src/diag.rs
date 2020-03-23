use annotate_snippets::{
    display_list::DisplayList,
    formatter::DisplayListFormatter,
    snippet::{Annotation, AnnotationType, Snippet, SourceAnnotation},
};
use escargot::format::diagnostic::{Diagnostic, DiagnosticLevel};

fn level_to_type(level: DiagnosticLevel) -> AnnotationType {
    match level {
        DiagnosticLevel::Warning => AnnotationType::Warning,
        DiagnosticLevel::Error => AnnotationType::Error,
        DiagnosticLevel::Note => AnnotationType::Note,
        DiagnosticLevel::Ice => AnnotationType::Info,
        DiagnosticLevel::Help => AnnotationType::Help,
        DiagnosticLevel::Unknown => AnnotationType::Info,
    }
}

fn diagnostic_to_snippet(diag: Diagnostic) -> Snippet {
    Snippet {
        title: Some(Annotation {
            label: Some(diag.message.clone().into_owned()),
            id: diag.code.clone().map(|code| code.code.clone().into_owned()),
            annotation_type: level_to_type(diag.level),
        }),
        footer: diag
            .children
            .iter()
            .filter(|child| child.spans.is_empty())
            .map(|child| Annotation {
                label: Some(child.message.clone().into_owned()),
                id: child
                    .code
                    .clone()
                    .map(|code| code.code.clone().into_owned()),
                annotation_type: level_to_type(child.level),
            })
            .collect(),
        slices: diag
            .spans
            .iter()
            .map(|span| annotate_snippets::snippet::Slice {
                source: span.text.iter().map(|text| &*text.text).collect(),
                line_start: span.line_start,
                origin: Some(span.file_name.display().to_string()),
                fold: false,
                annotations: std::iter::once(SourceAnnotation {
                    label: span
                        .label
                        .as_ref()
                        .map(|l| l.clone().into_owned())
                        .unwrap_or_else(String::new),
                    annotation_type: level_to_type(diag.level),
                    range: (span.column_start - 1, span.column_end - 1),
                })
                .chain(
                    diag.children
                        .iter()
                        .filter(|child| !child.spans.is_empty())
                        .flat_map(|child| {
                            child
                                .spans
                                .iter()
                                .map(|span| SourceAnnotation {
                                    label: child.message.clone().into_owned(),
                                    annotation_type: level_to_type(child.level),
                                    range: (span.column_start - 1, span.column_end - 1),
                                })
                                .collect::<Vec<_>>()
                        }),
                )
                .collect(),
            })
            .collect(),
    }
}

pub fn emit(diag: Diagnostic) {
    let snippet = diagnostic_to_snippet(diag);
    let snippet_formatter = DisplayListFormatter::new(true, false);
    println!("{}", snippet_formatter.format(&DisplayList::from(snippet)));
    println!();
}
