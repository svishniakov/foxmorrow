use foxmorrow_core::Error;
use foxmorrow_core::criteria::{parse_section, split_reference};

const TASK: &str = "# Задача\n\nОписание.\n\n## Критерии приёмки\n\n- GET /health возвращает 200.\n  - вложенный пункт не считается\n- Ответ содержит версию.\n\n```text\n- строка в блоке кода\n```\n\n### Подробности\n\n- пункт подраздела не входит\n\n## Риски\n\n- не входит\n";

#[test]
fn takes_flat_items_of_the_section_only() {
    let (_, criteria) = parse_section(TASK, "критерии приёмки").unwrap();
    let pairs: Vec<(&str, &str)> = criteria
        .iter()
        .map(|c| (c.id.as_str(), c.text.as_str()))
        .collect();
    assert_eq!(
        pairs,
        vec![
            ("ac-1", "GET /health возвращает 200."),
            ("ac-2", "Ответ содержит версию.")
        ]
    );
}

#[test]
fn returns_the_section_text_for_hashing() {
    let (section, _) = parse_section(TASK, "Критерии приёмки").unwrap();
    assert!(section.contains("- Ответ содержит версию."));
    assert!(!section.contains("Подробности"));
}

#[test]
fn accepts_numbered_lists() {
    let (_, criteria) = parse_section(
        "## Критерии приёмки\n\n1. Первый\n2. Второй\n",
        "Критерии приёмки",
    )
    .unwrap();
    assert_eq!(criteria.len(), 2);
    assert_eq!(criteria[1].text, "Второй");
}

#[test]
fn reports_missing_heading_and_empty_section() {
    assert!(matches!(
        parse_section(TASK, "Нет такого"),
        Err(Error::NotFound(_))
    ));
    assert!(matches!(
        parse_section("## Критерии приёмки\n\nТекст.\n", "Критерии приёмки"),
        Err(Error::Invalid(_))
    ));
}

#[test]
fn splits_file_and_heading() {
    assert_eq!(
        split_reference("docs/task.md#Критерии приёмки").unwrap(),
        ("docs/task.md", "Критерии приёмки")
    );
    assert!(split_reference("docs/task.md").is_err());
    assert!(split_reference("#heading").is_err());
}
