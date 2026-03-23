use ublx::utils::ClipboardCopyCommand;

#[test]
fn detect_returns_some_on_macos_or_when_tools_exist() {
    let c = ClipboardCopyCommand::detect();
    if cfg!(target_os = "macos") {
        assert!(c.is_some(), "pbcopy should be present on macOS");
    }
    if let Some(cmd) = c {
        assert!(!cmd.argv.is_empty());
    }
}
