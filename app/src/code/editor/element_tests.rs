use super::*;

#[test]
fn absolute_line_numbers_default_to_one_based_values() {
    assert_eq!(absolute_line_number(LineCount::from(0), None), 1);
    assert_eq!(absolute_line_number(LineCount::from(4), None), 5);
}

#[test]
fn absolute_line_numbers_honor_starting_line_number() {
    assert_eq!(absolute_line_number(LineCount::from(0), Some(10)), 10);
    assert_eq!(absolute_line_number(LineCount::from(4), Some(10)), 14);
}

#[test]
fn relative_line_numbers_show_absolute_value_on_active_line() {
    assert_eq!(
        display_line_number(
            LineCount::from(4),
            CodeEditorLineNumberMode::Relative,
            None,
            Some(LineCount::from(4)),
        ),
        5
    );
}

#[test]
fn relative_line_numbers_show_distance_above_and_below_active_line() {
    assert_eq!(
        display_line_number(
            LineCount::from(2),
            CodeEditorLineNumberMode::Relative,
            None,
            Some(LineCount::from(5)),
        ),
        3
    );
    assert_eq!(
        display_line_number(
            LineCount::from(8),
            CodeEditorLineNumberMode::Relative,
            None,
            Some(LineCount::from(5)),
        ),
        3
    );
}

#[test]
fn active_diff_range_uses_cursor_derived_range_without_focused_diff_navigation() {
    assert!(line_is_in_active_diff_range(
        LineCount::from(4),
        LineCount::from(5),
        Some(LineCount::from(4)..LineCount::from(7)),
        None,
    ));
}

#[test]
fn active_diff_range_keeps_lines_outside_cursor_hunk_absolute() {
    assert!(!line_is_in_active_diff_range(
        LineCount::from(8),
        LineCount::from(5),
        Some(LineCount::from(4)..LineCount::from(7)),
        Some(&(LineCount::from(8)..LineCount::from(10))),
    ));
}

#[test]
fn active_diff_range_can_fall_back_to_focused_range_when_it_contains_cursor() {
    assert!(line_is_in_active_diff_range(
        LineCount::from(6),
        LineCount::from(5),
        None,
        Some(&(LineCount::from(4)..LineCount::from(7))),
    ));
}

#[test]
fn active_diff_range_rejects_focused_range_that_does_not_contain_cursor() {
    assert!(!line_is_in_active_diff_range(
        LineCount::from(8),
        LineCount::from(5),
        None,
        Some(&(LineCount::from(8)..LineCount::from(10))),
    ));
}

#[test]
fn relative_line_numbers_fall_back_to_absolute_without_active_line() {
    assert_eq!(
        display_line_number(
            LineCount::from(4),
            CodeEditorLineNumberMode::Relative,
            None,
            None,
        ),
        5
    );
}

#[test]
fn relative_line_numbers_use_starting_line_number_for_active_line_only() {
    assert_eq!(
        display_line_number(
            LineCount::from(4),
            CodeEditorLineNumberMode::Relative,
            Some(10),
            Some(LineCount::from(4)),
        ),
        14
    );
    assert_eq!(
        display_line_number(
            LineCount::from(1),
            CodeEditorLineNumberMode::Relative,
            Some(10),
            Some(LineCount::from(4)),
        ),
        3
    );
}
