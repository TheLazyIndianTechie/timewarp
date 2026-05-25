use std::ffi::OsString;

use clap::Parser as _;
use clap_complete::aot::Shell;
use local_control::protocol::{ControlError, ErrorCode};
use serde_json::json;
use serial_test::serial;

use super::*;

const DISCOVERY_DIR_ENV: &str = "WARP_LOCAL_CONTROL_DISCOVERY_DIR";
fn parse_ok<const N: usize>(args: [&str; N]) -> ControlArgs {
    ControlArgs::try_parse_from(args).expect("command parses")
}

fn parse_err<const N: usize>(args: [&str; N]) -> clap::Error {
    ControlArgs::try_parse_from(args).expect_err("command is rejected")
}

fn set_discovery_dir(path: &std::path::Path) -> Option<OsString> {
    let previous = std::env::var_os(DISCOVERY_DIR_ENV);
    unsafe { std::env::set_var(DISCOVERY_DIR_ENV, path) };
    previous
}

fn restore_discovery_dir(previous: Option<OsString>) {
    match previous {
        Some(value) => unsafe { std::env::set_var(DISCOVERY_DIR_ENV, value) },
        None => unsafe { std::env::remove_var(DISCOVERY_DIR_ENV) },
    }
}
#[test]
fn parses_first_slice_tab_create() {
    let args = parse_ok(["warpctrl", "tab", "create", "--instance", "inst_123"]);
    let ControlCommand::Tab(TabCommand::Create(target)) = args.command else {
        panic!("expected tab create command");
    };
    assert_eq!(target.instance.as_deref(), Some("inst_123"));
}

#[test]
fn parses_first_slice_instance_list() {
    let args = parse_ok(["warpctrl", "instance", "list"]);
    assert!(matches!(
        args.command,
        ControlCommand::Instance(InstanceCommand::List)
    ));
}

#[test]
fn parses_first_slice_app_smoke_metadata_commands() {
    parse_ok(["warpctrl", "app", "ping"]);
    parse_ok(["warpctrl", "app", "version"]);
}

#[test]
fn parses_completion_generation_command() {
    let args = parse_ok(["warpctrl", "completions", "bash"]);
    assert!(matches!(
        args.command,
        ControlCommand::Completions {
            shell: Some(Shell::Bash)
        }
    ));
}

#[test]
fn rejects_future_catalog_commands_not_in_first_slice() {
    parse_err(["warpctrl", "window", "list"]);
    parse_err(["warpctrl", "tab", "list"]);
    parse_err(["warpctrl", "setting", "list"]);
}

#[test]
fn rejects_file_content_crud_commands() {
    for args in [
        ["warpctrl", "file", "read"],
        ["warpctrl", "file", "write"],
        ["warpctrl", "file", "append"],
        ["warpctrl", "file", "delete"],
    ] {
        parse_err(args);
    }
}

#[test]
fn parser_accepts_all_implemented_protocol_commands() {
    for args in [
        ["warpctrl", "instance", "list"],
        ["warpctrl", "app", "ping"],
        ["warpctrl", "app", "version"],
        ["warpctrl", "tab", "create"],
    ] {
        parse_ok(args);
    }
}

#[test]
fn instance_selector_flags_are_available_on_control_requests() {
    let args = parse_ok(["warpctrl", "app", "ping", "--pid", "1234"]);
    let ControlCommand::App(AppCommand::Ping(target)) = args.command else {
        panic!("expected app ping command");
    };
    assert_eq!(target.pid, Some(1234));

    parse_err([
        "warpctrl",
        "app",
        "ping",
        "--pid",
        "1234",
        "--instance",
        "inst_123",
    ]);
}

#[test]
fn generated_bash_completions_include_first_slice_commands() {
    let completions =
        generate_completion_string(Shell::Bash).expect("bash completions render to UTF-8");
    assert!(completions.contains("instance"));
    assert!(completions.contains("tab"));
    assert!(completions.contains("completions"));
}

#[test]
fn structured_error_output_uses_stable_error_code() {
    let error = ControlError::new(ErrorCode::NoInstance, "no local Warp control instances");
    let value = serde_json::to_value(ErrorSummary {
        ok: false,
        error: &error,
    })
    .expect("error summary serializes");
    assert_eq!(value["ok"], json!(false));
    assert_eq!(value["error"]["code"], json!("no_instance"));
    assert_eq!(
        value["error"]["message"],
        json!("no local Warp control instances")
    );
}

#[test]
#[serial]
fn tab_create_without_discovery_records_reports_no_instance() {
    let dir = std::env::temp_dir().join(format!(
        "warpctrl-empty-discovery-{}",
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&dir).expect("temp discovery dir is created");
    let previous = set_discovery_dir(&dir);
    let args =
        ControlArgs::try_parse_from(["warpctrl", "--output-format", "json", "tab", "create"])
            .expect("tab create parses");
    let error = run_inner(args).expect_err("missing instance is rejected");
    restore_discovery_dir(previous);
    assert_eq!(error.code, ErrorCode::NoInstance);
}
