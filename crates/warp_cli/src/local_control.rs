use std::io::Write as _;
use std::process::ExitCode;

use clap::{Args, CommandFactory, FromArgMatches, Parser, Subcommand, ValueEnum};
use clap_complete::aot::{Shell, generate};
use local_control::protocol::{
    Action, ActionKind, ActionMetadata, AppFocusParams, AppSurfaceParams, AppearanceFontSizeParams,
    AppearanceSetParams, AppearanceZoomParams, ControlError, DriveCreateParams, DriveDeleteParams,
    DriveInsertParams, DriveRunParams, DriveUpdateParams, ErrorCode, FileDeleteParams,
    FileOpenParams, FileWriteParams, HorizontalDirection, InputClearParams, InputInsertParams,
    InputMode, InputModeSetParams, InputReplaceParams, InputRunParams, PaneCloseParams,
    PaneDirection, PaneFocusParams, PaneMaximizeParams, PaneNavigateParams, PaneResizeParams,
    PaneSplitParams, SettingSetParams, SettingToggleParams, SizeAdjustment, TabActivateParams,
    TabActivationTarget, TabCloseParams, TabCloseScope, TabMoveParams, TabRenameParams,
    ThemeSetParams, WindowCloseParams, WindowCreateParams, WindowFocusParams,
};
use local_control::selection::{InstanceSelector, select_instance};
use serde::Serialize;

use crate::agent::OutputFormat;

#[derive(Debug, Parser)]
#[command(
    name = "warpctrl",
    display_name = "warpctrl",
    about = "Control a running local Warp app instance"
)]
pub struct ControlArgs {
    /// Set the output format.
    #[arg(
        long = "output-format",
        global = true,
        value_enum,
        default_value_t = OutputFormat::Pretty,
        env = "WARP_OUTPUT_FORMAT"
    )]
    pub output_format: OutputFormat,

    #[command(subcommand)]
    pub command: ControlCommand,
}

impl ControlArgs {
    pub fn from_env() -> Self {
        let matches = Self::clap_command().get_matches();
        Self::from_arg_matches(&matches).unwrap_or_else(|err| err.exit())
    }

    pub fn clap_command() -> clap::Command {
        let bin_name = crate::binary_name().unwrap_or_else(|| "warpctrl".to_owned());
        <Self as CommandFactory>::command()
            .version(crate::version_string())
            .bin_name(bin_name.clone())
            .after_help(color_print::cformat!(
                r#"<bold><underline>Examples:</underline></bold>

  <dim>$</dim> <bold>{bin_name} instance list</bold>

  <dim>$</dim> <bold>{bin_name} tab create</bold>

<bold><underline>Learn more:</underline></bold>
* Use <bold>{bin_name} help</bold> to learn more about each command
"#
            ))
    }
}

fn run_app_surface_command(
    args: AppSurfaceArgs,
    action: ActionKind,
    output_format: OutputFormat,
) -> Result<(), ControlError> {
    run_action_with_params(
        args.target,
        action,
        AppSurfaceParams {
            query: args.query,
            page: args.page,
        },
        output_format,
    )
}

fn run_tab_activate_relative(
    args: TargetArgs,
    relative: TabActivationTarget,
    output_format: OutputFormat,
) -> Result<(), ControlError> {
    run_action_with_params(
        args,
        ActionKind::TabActivate,
        TabActivateParams {
            relative: Some(relative),
        },
        output_format,
    )
}

fn parse_json_value_or_string(value: String) -> serde_json::Value {
    match serde_json::from_str(&value) {
        Ok(value) => value,
        Err(_) => serde_json::Value::String(value),
    }
}

#[derive(Debug, Clone, Subcommand)]
pub enum ControlCommand {
    /// Inspect local Warp app instances.
    #[command(subcommand)]
    Instance(InstanceCommand),
    /// Inspect a selected local Warp app.
    #[command(subcommand)]
    App(AppCommand),
    /// Inspect the local-control action catalog.
    #[command(subcommand)]
    Action(ActionCommand),
    /// Inspect local Warp windows.
    #[command(subcommand)]
    Window(WindowCommand),
    /// Control local Warp tabs.
    #[command(subcommand)]
    Tab(TabCommand),
    /// Inspect local Warp panes.
    #[command(subcommand)]
    Pane(PaneCommand),
    /// Inspect local Warp sessions.
    #[command(subcommand)]
    Session(SessionCommand),
    /// Inspect terminal blocks.
    #[command(subcommand)]
    Block(BlockCommand),
    /// Inspect terminal input state.
    #[command(subcommand)]
    Input(InputCommand),
    /// Inspect terminal command history.
    #[command(subcommand)]
    History(HistoryCommand),
    /// Inspect Warp themes.
    #[command(subcommand)]
    Theme(ThemeCommand),
    /// Inspect appearance state.
    #[command(subcommand)]
    Appearance(AppearanceCommand),
    /// Inspect allowlisted settings.
    #[command(subcommand)]
    Setting(SettingCommand),
    /// Inspect files currently surfaced in Warp.
    #[command(subcommand)]
    File(FileCommand),
    /// Inspect projects currently known to Warp.
    #[command(subcommand)]
    Project(ProjectCommand),
    /// Inspect Warp Drive objects.
    #[command(subcommand)]
    Drive(DriveCommand),

    /// Generate shell completions for your shell to stdout.
    ///
    /// For bash, add the following to ~/.bashrc:
    ///     source <(path/to/warpctrl completions bash)
    ///
    /// For zsh, add the following to ~/.zshrc:
    ///     source <(path/to/warpctrl completions zsh)
    ///
    /// For fish, add the following to ~/.config/fish/config.fish:
    ///     path/to/warpctrl completions fish | source
    ///
    /// For Powershell, add the following to $PROFILE:
    ///     path\to\warpctrl completions powershell | Out-String | Invoke-Expression
    ///
    /// If no shell is provided, this defaults to the shell that Warp was run from.
    #[command(verbatim_doc_comment)]
    Completions {
        /// Shell to generate completions for.
        #[arg(value_enum)]
        shell: Option<Shell>,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum InstanceCommand {
    /// List locally discoverable Warp instances.
    List,
}

#[derive(Debug, Clone, Subcommand)]
pub enum AppCommand {
    /// Check that the selected local Warp app responds.
    Ping(TargetArgs),
    /// Print protocol and app version metadata for the selected local Warp app.
    Version(TargetArgs),
    /// Print the active window/tab/pane/session chain.
    Active(TargetArgs),
    /// Print app and protocol metadata.
    Inspect(TargetArgs),
    /// Focus the selected Warp app instance.
    Focus(TargetArgs),
    /// Open the Settings surface.
    SettingsOpen(AppSurfaceArgs),
    /// Open the Command Palette.
    CommandPaletteOpen(AppSurfaceArgs),
    /// Open command search.
    CommandSearchOpen(AppSurfaceArgs),
    /// Open Warp Drive.
    WarpDriveOpen(AppSurfaceArgs),
    /// Toggle Warp Drive.
    WarpDriveToggle(AppSurfaceArgs),
    /// Toggle the resource center.
    ResourceCenterToggle(AppSurfaceArgs),
    /// Toggle the AI assistant surface.
    AiAssistantToggle(AppSurfaceArgs),
    /// Toggle the code review surface.
    CodeReviewToggle(AppSurfaceArgs),
    /// Toggle the vertical tabs panel.
    VerticalTabsToggle(AppSurfaceArgs),
}

#[derive(Debug, Clone, Subcommand)]
pub enum ActionCommand {
    /// List allowlisted local-control actions.
    List(TargetArgs),
    /// Inspect one allowlisted local-control action.
    Get(ActionGetArgs),
}

#[derive(Debug, Clone, Subcommand)]
pub enum WindowCommand {
    /// List windows in the selected local Warp app.
    List(TargetArgs),
    /// Create a new Warp window.
    Create(WindowCreateArgs),
    /// Focus a Warp window.
    Focus(TargetArgs),
    /// Close a Warp window.
    Close(WindowCloseArgs),
}

#[derive(Debug, Clone, Subcommand)]
pub enum TabCommand {
    /// List tabs in the selected local Warp app.
    List(TargetArgs),
    /// Create a new terminal tab in the active window.
    Create(TargetArgs),
    /// Activate a target tab.
    Activate(TargetArgs),
    /// Activate the previous tab.
    Previous(TargetArgs),
    /// Activate the next tab.
    Next(TargetArgs),
    /// Activate the last tab.
    Last(TargetArgs),
    /// Move a target tab left or right.
    Move(TabMoveArgs),
    /// Rename or reset a target tab title.
    Rename(TabRenameArgs),
    /// Close a target tab or tab group.
    Close(TabCloseArgs),
}

#[derive(Debug, Clone, Subcommand)]
pub enum PaneCommand {
    /// List panes in the selected local Warp app.
    List(TargetArgs),
    /// Split a pane.
    Split(PaneSplitArgs),
    /// Focus a pane.
    Focus(TargetArgs),
    /// Navigate pane focus.
    Navigate(PaneNavigateArgs),
    /// Close a pane.
    Close(PaneCloseArgs),
    /// Toggle or set pane maximization.
    Maximize(PaneMaximizeArgs),
    /// Resize a pane divider.
    Resize(PaneResizeArgs),
    /// Switch to the previous session in a pane.
    PreviousSession(TargetArgs),
    /// Switch to the next session in a pane.
    NextSession(TargetArgs),
}

#[derive(Debug, Clone, Subcommand)]
pub enum SessionCommand {
    /// List sessions in the selected local Warp app.
    List(TargetArgs),
}

#[derive(Debug, Clone, Subcommand)]
pub enum BlockCommand {
    /// List terminal blocks.
    List(LimitTargetArgs),
    /// Read one terminal block.
    Get(BlockGetArgs),
}

#[derive(Debug, Clone, Subcommand)]
pub enum InputCommand {
    /// Read the current input buffer.
    Get(TargetArgs),
    /// Insert text into the active input buffer.
    Insert(InputInsertArgs),
    /// Replace the active input buffer.
    Replace(InputTextArgs),
    /// Clear the active input buffer.
    Clear(TargetArgs),
    /// Set the active input mode.
    Mode(InputModeArgs),
    /// Run a command in the target session.
    Run(InputRunArgs),
}

#[derive(Debug, Clone, Subcommand)]
pub enum HistoryCommand {
    /// List command history entries.
    List(LimitTargetArgs),
}

#[derive(Debug, Clone, Subcommand)]
pub enum ThemeCommand {
    /// List available themes.
    List(TargetArgs),
    /// Set the current theme.
    Set(ThemeSetArgs),
}

#[derive(Debug, Clone, Subcommand)]
pub enum AppearanceCommand {
    /// Read appearance state.
    Get(TargetArgs),
    /// Set theme-following appearance state.
    Set(AppearanceSetArgs),
    /// Adjust font size.
    FontSize(AppearanceAdjustArgs),
    /// Adjust UI zoom.
    Zoom(AppearanceAdjustArgs),
}

#[derive(Debug, Clone, Subcommand)]
pub enum SettingCommand {
    /// List allowlisted settings.
    List(TargetArgs),
    /// Read one allowlisted setting.
    Get(SettingGetArgs),
    /// Set one allowlisted setting.
    Set(SettingSetArgsCli),
    /// Toggle one allowlisted boolean setting.
    Toggle(SettingToggleArgsCli),
}

#[derive(Debug, Clone, Subcommand)]
pub enum FileCommand {
    /// List files currently surfaced in Warp.
    List(TargetArgs),
    /// Open a path in Warp.
    Open(FileOpenArgs),
    /// Write a file through the local-control protocol.
    Write(FileWriteArgs),
    /// Delete a file through the local-control protocol.
    Delete(FileDeleteArgs),
}

#[derive(Debug, Clone, Subcommand)]
pub enum ProjectCommand {
    /// Print the active project for the selected local Warp app.
    Active(TargetArgs),
    /// List projects currently known to Warp.
    List(TargetArgs),
}

#[derive(Debug, Clone, Subcommand)]
pub enum DriveCommand {
    /// List Warp Drive objects.
    List(DriveListArgs),
    /// Read one Warp Drive object.
    Get(DriveGetArgs),
    /// Create a Warp Drive object.
    Create(DriveCreateArgs),
    /// Update a Warp Drive object.
    Update(DriveUpdateArgs),
    /// Delete a Warp Drive object.
    Delete(DriveObjectMutationArgs),
    /// Run a Warp Drive workflow.
    Run(DriveObjectMutationArgs),
    /// Insert a Warp Drive object into the active input.
    Insert(DriveObjectMutationArgs),
}

#[derive(Debug, Clone, Args, Default)]
pub struct TargetArgs {
    /// Target a specific local Warp instance id from `warp instance list`.
    #[arg(long = "instance")]
    pub instance: Option<String>,

    /// Target a specific local Warp process id.
    #[arg(long = "pid", conflicts_with = "instance")]
    pub pid: Option<u32>,
}

#[derive(Debug, Clone, Args)]
pub struct AppSurfaceArgs {
    #[command(flatten)]
    pub target: TargetArgs,

    #[arg(long = "query")]
    pub query: Option<String>,

    #[arg(long = "page")]
    pub page: Option<String>,
}

#[derive(Debug, Clone, Args)]
pub struct WindowCreateArgs {
    #[command(flatten)]
    pub target: TargetArgs,

    #[arg(long = "profile")]
    pub profile: Option<String>,
}

#[derive(Debug, Clone, Args)]
pub struct WindowCloseArgs {
    #[command(flatten)]
    pub target: TargetArgs,

    #[arg(long = "force")]
    pub force: bool,
}

#[derive(Debug, Clone, Args)]
pub struct TabMoveArgs {
    #[command(flatten)]
    pub target: TargetArgs,

    #[arg(long = "direction", value_enum)]
    pub direction: HorizontalDirectionArg,
}

#[derive(Debug, Clone, Args)]
pub struct TabRenameArgs {
    #[command(flatten)]
    pub target: TargetArgs,

    pub title: Option<String>,

    #[arg(long = "reset")]
    pub reset: bool,
}

#[derive(Debug, Clone, Args)]
pub struct TabCloseArgs {
    #[command(flatten)]
    pub target: TargetArgs,

    #[arg(long = "scope", value_enum, default_value_t = TabCloseScopeArg::Target)]
    pub scope: TabCloseScopeArg,

    #[arg(long = "force")]
    pub force: bool,
}

#[derive(Debug, Clone, Args)]
pub struct PaneSplitArgs {
    #[command(flatten)]
    pub target: TargetArgs,

    #[arg(long = "direction", value_enum)]
    pub direction: PaneDirectionArg,

    #[arg(long = "profile")]
    pub profile: Option<String>,
}

#[derive(Debug, Clone, Args)]
pub struct PaneNavigateArgs {
    #[command(flatten)]
    pub target: TargetArgs,

    #[arg(long = "direction", value_enum)]
    pub direction: PaneDirectionArg,
}

#[derive(Debug, Clone, Args)]
pub struct PaneCloseArgs {
    #[command(flatten)]
    pub target: TargetArgs,

    #[arg(long = "force")]
    pub force: bool,
}

#[derive(Debug, Clone, Args)]
pub struct PaneMaximizeArgs {
    #[command(flatten)]
    pub target: TargetArgs,

    #[arg(long = "enabled")]
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Args)]
pub struct PaneResizeArgs {
    #[command(flatten)]
    pub target: TargetArgs,

    #[arg(long = "direction", value_enum)]
    pub direction: PaneDirectionArg,

    #[arg(long = "amount")]
    pub amount: Option<u32>,
}

#[derive(Debug, Clone, Args)]
pub struct InputInsertArgs {
    #[command(flatten)]
    pub target: TargetArgs,

    pub text: String,

    #[arg(long = "replace")]
    pub replace: bool,
}

#[derive(Debug, Clone, Args)]
pub struct InputTextArgs {
    #[command(flatten)]
    pub target: TargetArgs,

    pub text: String,
}

#[derive(Debug, Clone, Args)]
pub struct InputModeArgs {
    #[command(flatten)]
    pub target: TargetArgs,

    #[arg(value_enum)]
    pub mode: InputModeArg,
}

#[derive(Debug, Clone, Args)]
pub struct InputRunArgs {
    #[command(flatten)]
    pub target: TargetArgs,

    pub command: String,
}

#[derive(Debug, Clone, Args)]
pub struct ThemeSetArgs {
    #[command(flatten)]
    pub target: TargetArgs,

    pub name: String,
}

#[derive(Debug, Clone, Args)]
pub struct AppearanceSetArgs {
    #[command(flatten)]
    pub target: TargetArgs,

    #[arg(long = "theme")]
    pub theme: Option<String>,

    #[arg(long = "follow-system-theme")]
    pub follow_system_theme: Option<bool>,

    #[arg(long = "light-theme")]
    pub light_theme: Option<String>,

    #[arg(long = "dark-theme")]
    pub dark_theme: Option<String>,
}

#[derive(Debug, Clone, Args)]
pub struct AppearanceAdjustArgs {
    #[command(flatten)]
    pub target: TargetArgs,

    #[arg(value_enum)]
    pub adjustment: SizeAdjustmentArg,

    #[arg(long = "value")]
    pub value: Option<u32>,
}

#[derive(Debug, Clone, Args)]
pub struct SettingSetArgsCli {
    #[command(flatten)]
    pub target: TargetArgs,

    pub key: String,

    pub value: String,
}

#[derive(Debug, Clone, Args)]
pub struct SettingToggleArgsCli {
    #[command(flatten)]
    pub target: TargetArgs,

    pub key: String,
}

#[derive(Debug, Clone, Args)]
pub struct FileOpenArgs {
    #[command(flatten)]
    pub target: TargetArgs,

    pub path: String,

    #[arg(long = "line")]
    pub line: Option<u32>,

    #[arg(long = "new-window")]
    pub new_window: bool,
}

#[derive(Debug, Clone, Args)]
pub struct FileWriteArgs {
    #[command(flatten)]
    pub target: TargetArgs,

    pub path: String,

    pub contents: String,

    #[arg(long = "create")]
    pub create: bool,
}

#[derive(Debug, Clone, Args)]
pub struct FileDeleteArgs {
    #[command(flatten)]
    pub target: TargetArgs,

    pub path: String,

    #[arg(long = "recursive")]
    pub recursive: bool,
}

#[derive(Debug, Clone, Args)]
pub struct ActionGetArgs {
    #[command(flatten)]
    pub target: TargetArgs,

    /// Action name, such as tab.create or window.list.
    pub action: String,
}

#[derive(Debug, Clone, Args)]
pub struct LimitTargetArgs {
    #[command(flatten)]
    pub target: TargetArgs,

    /// Maximum number of items to return.
    #[arg(long = "limit")]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Args)]
pub struct BlockGetArgs {
    #[command(flatten)]
    pub target: TargetArgs,

    /// Opaque block id returned by block list.
    pub block_id: String,
}

#[derive(Debug, Clone, Args)]
pub struct SettingGetArgs {
    #[command(flatten)]
    pub target: TargetArgs,

    /// Allowlisted setting key.
    pub key: String,
}

#[derive(Debug, Clone, Args)]
pub struct DriveListArgs {
    #[command(flatten)]
    pub target: TargetArgs,

    /// Restrict results to one Drive object type.
    #[arg(long = "type")]
    pub object_type: Option<DriveObjectTypeArg>,
}

#[derive(Debug, Clone, Args)]
pub struct DriveGetArgs {
    #[command(flatten)]
    pub target: TargetArgs,

    /// Warp Drive object type.
    #[arg(long = "type")]
    pub object_type: DriveObjectTypeArg,

    /// Opaque Warp Drive object id.
    pub id: String,
}
#[derive(Debug, Clone, Args)]
pub struct DriveCreateArgs {
    #[command(flatten)]
    pub target: TargetArgs,

    /// Warp Drive object type.
    #[arg(long = "type")]
    pub object_type: DriveObjectTypeArg,

    /// Name for the new Drive object.
    pub name: String,

    /// Object content, parsed as JSON when possible and otherwise treated as a string.
    pub content: String,
}

#[derive(Debug, Clone, Args)]
pub struct DriveUpdateArgs {
    #[command(flatten)]
    pub target: TargetArgs,

    /// Warp Drive object type.
    #[arg(long = "type")]
    pub object_type: DriveObjectTypeArg,

    /// Opaque Warp Drive object id.
    pub id: String,

    /// Object content, parsed as JSON when possible and otherwise treated as a string.
    pub content: String,
}

#[derive(Debug, Clone, Args)]
pub struct DriveObjectMutationArgs {
    #[command(flatten)]
    pub target: TargetArgs,

    /// Warp Drive object type.
    #[arg(long = "type")]
    pub object_type: DriveObjectTypeArg,

    /// Opaque Warp Drive object id.
    pub id: String,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum DriveObjectTypeArg {
    Workflow,
    Notebook,
    Environment,
    Prompt,
}

impl From<DriveObjectTypeArg> for local_control::DriveObjectType {
    fn from(value: DriveObjectTypeArg) -> Self {
        match value {
            DriveObjectTypeArg::Workflow => Self::Workflow,
            DriveObjectTypeArg::Notebook => Self::Notebook,
            DriveObjectTypeArg::Environment => Self::Environment,
            DriveObjectTypeArg::Prompt => Self::Prompt,
        }
    }
}
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum HorizontalDirectionArg {
    Left,
    Right,
}

impl From<HorizontalDirectionArg> for HorizontalDirection {
    fn from(value: HorizontalDirectionArg) -> Self {
        match value {
            HorizontalDirectionArg::Left => Self::Left,
            HorizontalDirectionArg::Right => Self::Right,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum TabCloseScopeArg {
    Target,
    Others,
    Right,
}

impl From<TabCloseScopeArg> for TabCloseScope {
    fn from(value: TabCloseScopeArg) -> Self {
        match value {
            TabCloseScopeArg::Target => Self::Target,
            TabCloseScopeArg::Others => Self::Others,
            TabCloseScopeArg::Right => Self::Right,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum PaneDirectionArg {
    Left,
    Right,
    Up,
    Down,
}

impl From<PaneDirectionArg> for PaneDirection {
    fn from(value: PaneDirectionArg) -> Self {
        match value {
            PaneDirectionArg::Left => Self::Left,
            PaneDirectionArg::Right => Self::Right,
            PaneDirectionArg::Up => Self::Up,
            PaneDirectionArg::Down => Self::Down,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum InputModeArg {
    Terminal,
    Agent,
}

impl From<InputModeArg> for InputMode {
    fn from(value: InputModeArg) -> Self {
        match value {
            InputModeArg::Terminal => Self::Terminal,
            InputModeArg::Agent => Self::Agent,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum SizeAdjustmentArg {
    Increase,
    Decrease,
    Reset,
    Set,
}

impl From<SizeAdjustmentArg> for SizeAdjustment {
    fn from(value: SizeAdjustmentArg) -> Self {
        match value {
            SizeAdjustmentArg::Increase => Self::Increase,
            SizeAdjustmentArg::Decrease => Self::Decrease,
            SizeAdjustmentArg::Reset => Self::Reset,
            SizeAdjustmentArg::Set => Self::Set,
        }
    }
}

#[derive(Serialize)]
struct InstanceSummary {
    instance_id: String,
    pid: u32,
    channel: String,
    app_id: String,
    app_version: Option<String>,
    started_at: String,
    endpoint: Option<local_control::discovery::ControlEndpoint>,
    outside_warp_control_enabled: bool,
    actions: Vec<ActionMetadata>,
}

impl From<local_control::discovery::InstanceRecord> for InstanceSummary {
    fn from(record: local_control::discovery::InstanceRecord) -> Self {
        Self {
            instance_id: record.instance_id.0,
            pid: record.pid,
            channel: record.channel,
            app_id: record.app_id,
            app_version: record.app_version,
            started_at: record.started_at.to_rfc3339(),
            endpoint: record.endpoint,
            outside_warp_control_enabled: record.outside_warp_control_enabled,
            actions: record.actions,
        }
    }
}

#[derive(Serialize)]
struct ErrorSummary<'a> {
    ok: bool,
    error: &'a ControlError,
}

pub fn run(args: ControlArgs) -> ExitCode {
    let output_format = args.output_format;
    match run_inner(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            if let Err(write_error) = write_control_error(&error, output_format) {
                eprintln!(
                    "error: failed to render local-control error: {}",
                    write_error.message
                );
            }
            ExitCode::FAILURE
        }
    }
}

fn run_inner(args: ControlArgs) -> Result<(), ControlError> {
    let output_format = args.output_format;
    match args.command {
        ControlCommand::Instance(command) => run_instance_command(command, output_format),
        ControlCommand::App(command) => run_app_command(command, output_format),
        ControlCommand::Action(command) => run_action_command(command, output_format),
        ControlCommand::Window(command) => run_window_command(command, output_format),
        ControlCommand::Tab(command) => run_tab_command(command, output_format),
        ControlCommand::Pane(command) => run_pane_command(command, output_format),
        ControlCommand::Session(command) => run_session_command(command, output_format),
        ControlCommand::Block(command) => run_block_command(command, output_format),
        ControlCommand::Input(command) => run_input_command(command, output_format),
        ControlCommand::History(command) => run_history_command(command, output_format),
        ControlCommand::Theme(command) => run_theme_command(command, output_format),
        ControlCommand::Appearance(command) => run_appearance_command(command, output_format),
        ControlCommand::Setting(command) => run_setting_command(command, output_format),
        ControlCommand::File(command) => run_file_command(command, output_format),
        ControlCommand::Project(command) => run_project_command(command, output_format),
        ControlCommand::Drive(command) => run_drive_command(command, output_format),
        ControlCommand::Completions { shell } => generate_completions_to_stdout(shell),
    }
}

fn run_instance_command(
    command: InstanceCommand,
    output_format: OutputFormat,
) -> Result<(), ControlError> {
    match command {
        InstanceCommand::List => {
            let summaries = local_control::discovery::list_instances()
                .into_iter()
                .map(InstanceSummary::from)
                .collect::<Vec<_>>();
            match output_format {
                OutputFormat::Json => write_json(&summaries),
                OutputFormat::Ndjson => {
                    for summary in summaries {
                        write_json_line(&summary)?;
                    }
                    Ok(())
                }
                OutputFormat::Pretty | OutputFormat::Text => {
                    for summary in summaries {
                        let endpoint = summary
                            .endpoint
                            .as_ref()
                            .map(|endpoint| format!("{}:{}", endpoint.host, endpoint.port))
                            .unwrap_or_else(|| "outside_warp_disabled".to_owned());
                        println!(
                            "{}\tpid={}\t{}\t{}",
                            summary.instance_id, summary.pid, summary.channel, endpoint
                        );
                    }
                    Ok(())
                }
            }
        }
    }
}

fn run_app_command(command: AppCommand, output_format: OutputFormat) -> Result<(), ControlError> {
    match command {
        AppCommand::Ping(args) => run_action_with_params(
            args,
            ActionKind::AppPing,
            local_control::EmptyParams {},
            output_format,
        ),
        AppCommand::Version(args) => run_action_with_params(
            args,
            ActionKind::AppVersion,
            local_control::EmptyParams {},
            output_format,
        ),
        AppCommand::Active(args) => run_action_with_params(
            args,
            ActionKind::AppActive,
            local_control::AppActiveParams::default(),
            output_format,
        ),
        AppCommand::Inspect(args) => run_action_with_params(
            args,
            ActionKind::AppInspect,
            local_control::AppInspectParams::default(),
            output_format,
        ),
        AppCommand::Focus(args) => run_action_with_params(
            args,
            ActionKind::AppFocus,
            AppFocusParams::default(),
            output_format,
        ),
        AppCommand::SettingsOpen(args) => {
            run_app_surface_command(args, ActionKind::AppSettingsOpen, output_format)
        }
        AppCommand::CommandPaletteOpen(args) => {
            run_app_surface_command(args, ActionKind::AppCommandPaletteOpen, output_format)
        }
        AppCommand::CommandSearchOpen(args) => {
            run_app_surface_command(args, ActionKind::AppCommandSearchOpen, output_format)
        }
        AppCommand::WarpDriveOpen(args) => {
            run_app_surface_command(args, ActionKind::AppWarpDriveOpen, output_format)
        }
        AppCommand::WarpDriveToggle(args) => {
            run_app_surface_command(args, ActionKind::AppWarpDriveToggle, output_format)
        }
        AppCommand::ResourceCenterToggle(args) => {
            run_app_surface_command(args, ActionKind::AppResourceCenterToggle, output_format)
        }
        AppCommand::AiAssistantToggle(args) => {
            run_app_surface_command(args, ActionKind::AppAiAssistantToggle, output_format)
        }
        AppCommand::CodeReviewToggle(args) => {
            run_app_surface_command(args, ActionKind::AppCodeReviewToggle, output_format)
        }
        AppCommand::VerticalTabsToggle(args) => {
            run_app_surface_command(args, ActionKind::AppVerticalTabsToggle, output_format)
        }
    }
}

fn run_action_command(
    command: ActionCommand,
    output_format: OutputFormat,
) -> Result<(), ControlError> {
    match command {
        ActionCommand::List(args) => run_action_with_params(
            args,
            ActionKind::ActionList,
            local_control::ActionListParams::default(),
            output_format,
        ),
        ActionCommand::Get(args) => run_action_with_params(
            args.target,
            ActionKind::ActionGet,
            local_control::ActionGetParams {
                action: args.action,
            },
            output_format,
        ),
    }
}

fn run_window_command(
    command: WindowCommand,
    output_format: OutputFormat,
) -> Result<(), ControlError> {
    match command {
        WindowCommand::List(args) => run_action_with_params(
            args,
            ActionKind::WindowList,
            local_control::EmptyParams {},
            output_format,
        ),
        WindowCommand::Create(args) => run_action_with_params(
            args.target,
            ActionKind::WindowCreate,
            WindowCreateParams {
                profile: args.profile,
            },
            output_format,
        ),
        WindowCommand::Focus(args) => run_action_with_params(
            args,
            ActionKind::WindowFocus,
            WindowFocusParams::default(),
            output_format,
        ),
        WindowCommand::Close(args) => run_action_with_params(
            args.target,
            ActionKind::WindowClose,
            WindowCloseParams { force: args.force },
            output_format,
        ),
    }
}

fn run_tab_command(command: TabCommand, output_format: OutputFormat) -> Result<(), ControlError> {
    match command {
        TabCommand::List(args) => run_action_with_params(
            args,
            ActionKind::TabList,
            local_control::EmptyParams {},
            output_format,
        ),
        TabCommand::Create(args) => run_action_with_params(
            args,
            ActionKind::TabCreate,
            local_control::EmptyParams {},
            output_format,
        ),
        TabCommand::Activate(args) => run_action_with_params(
            args,
            ActionKind::TabActivate,
            TabActivateParams { relative: None },
            output_format,
        ),
        TabCommand::Previous(args) => {
            run_tab_activate_relative(args, TabActivationTarget::Previous, output_format)
        }
        TabCommand::Next(args) => {
            run_tab_activate_relative(args, TabActivationTarget::Next, output_format)
        }
        TabCommand::Last(args) => {
            run_tab_activate_relative(args, TabActivationTarget::Last, output_format)
        }
        TabCommand::Move(args) => run_action_with_params(
            args.target,
            ActionKind::TabMove,
            TabMoveParams {
                direction: args.direction.into(),
            },
            output_format,
        ),
        TabCommand::Rename(args) => run_action_with_params(
            args.target,
            ActionKind::TabRename,
            TabRenameParams {
                title: if args.reset { None } else { args.title },
            },
            output_format,
        ),
        TabCommand::Close(args) => run_action_with_params(
            args.target,
            ActionKind::TabClose,
            TabCloseParams {
                scope: args.scope.into(),
                force: args.force,
            },
            output_format,
        ),
    }
}

fn run_pane_command(command: PaneCommand, output_format: OutputFormat) -> Result<(), ControlError> {
    match command {
        PaneCommand::List(args) => run_action_with_params(
            args,
            ActionKind::PaneList,
            local_control::EmptyParams {},
            output_format,
        ),
        PaneCommand::Split(args) => run_action_with_params(
            args.target,
            ActionKind::PaneSplit,
            PaneSplitParams {
                direction: args.direction.into(),
                profile: args.profile,
            },
            output_format,
        ),
        PaneCommand::Focus(args) => run_action_with_params(
            args,
            ActionKind::PaneFocus,
            PaneFocusParams::default(),
            output_format,
        ),
        PaneCommand::Navigate(args) => run_action_with_params(
            args.target,
            ActionKind::PaneNavigate,
            PaneNavigateParams {
                direction: args.direction.into(),
            },
            output_format,
        ),
        PaneCommand::Close(args) => run_action_with_params(
            args.target,
            ActionKind::PaneClose,
            PaneCloseParams { force: args.force },
            output_format,
        ),
        PaneCommand::Maximize(args) => run_action_with_params(
            args.target,
            ActionKind::PaneMaximize,
            PaneMaximizeParams {
                enabled: args.enabled,
            },
            output_format,
        ),
        PaneCommand::Resize(args) => run_action_with_params(
            args.target,
            ActionKind::PaneResize,
            PaneResizeParams {
                direction: args.direction.into(),
                amount: args.amount,
            },
            output_format,
        ),
        PaneCommand::PreviousSession(args) => run_action_with_params(
            args,
            ActionKind::PaneSessionPrevious,
            local_control::EmptyParams {},
            output_format,
        ),
        PaneCommand::NextSession(args) => run_action_with_params(
            args,
            ActionKind::PaneSessionNext,
            local_control::EmptyParams {},
            output_format,
        ),
    }
}

fn run_session_command(
    command: SessionCommand,
    output_format: OutputFormat,
) -> Result<(), ControlError> {
    match command {
        SessionCommand::List(args) => run_action_with_params(
            args,
            ActionKind::SessionList,
            local_control::EmptyParams {},
            output_format,
        ),
    }
}

fn run_block_command(
    command: BlockCommand,
    output_format: OutputFormat,
) -> Result<(), ControlError> {
    match command {
        BlockCommand::List(args) => run_action_with_params(
            args.target,
            ActionKind::BlockList,
            local_control::BlockListParams { limit: args.limit },
            output_format,
        ),
        BlockCommand::Get(args) => run_action_with_params(
            args.target,
            ActionKind::BlockGet,
            local_control::BlockGetParams {
                block_id: args.block_id,
            },
            output_format,
        ),
    }
}

fn run_input_command(
    command: InputCommand,
    output_format: OutputFormat,
) -> Result<(), ControlError> {
    match command {
        InputCommand::Get(args) => run_action_with_params(
            args,
            ActionKind::InputGet,
            local_control::InputGetParams::default(),
            output_format,
        ),
        InputCommand::Insert(args) => run_action_with_params(
            args.target,
            ActionKind::InputInsert,
            InputInsertParams {
                text: args.text,
                replace: args.replace,
            },
            output_format,
        ),
        InputCommand::Replace(args) => run_action_with_params(
            args.target,
            ActionKind::InputReplace,
            InputReplaceParams { text: args.text },
            output_format,
        ),
        InputCommand::Clear(args) => run_action_with_params(
            args,
            ActionKind::InputClear,
            InputClearParams::default(),
            output_format,
        ),
        InputCommand::Mode(args) => run_action_with_params(
            args.target,
            ActionKind::InputModeSet,
            InputModeSetParams {
                mode: args.mode.into(),
            },
            output_format,
        ),
        InputCommand::Run(args) => run_action_with_params(
            args.target,
            ActionKind::InputRun,
            InputRunParams {
                command: args.command,
            },
            output_format,
        ),
    }
}

fn run_history_command(
    command: HistoryCommand,
    output_format: OutputFormat,
) -> Result<(), ControlError> {
    match command {
        HistoryCommand::List(args) => run_action_with_params(
            args.target,
            ActionKind::HistoryList,
            local_control::HistoryListParams { limit: args.limit },
            output_format,
        ),
    }
}

fn run_theme_command(
    command: ThemeCommand,
    output_format: OutputFormat,
) -> Result<(), ControlError> {
    match command {
        ThemeCommand::List(args) => run_action_with_params(
            args,
            ActionKind::ThemeList,
            local_control::EmptyParams {},
            output_format,
        ),
        ThemeCommand::Set(args) => run_action_with_params(
            args.target,
            ActionKind::ThemeSet,
            ThemeSetParams { name: args.name },
            output_format,
        ),
    }
}

fn run_appearance_command(
    command: AppearanceCommand,
    output_format: OutputFormat,
) -> Result<(), ControlError> {
    match command {
        AppearanceCommand::Get(args) => run_action_with_params(
            args,
            ActionKind::AppearanceGet,
            local_control::EmptyParams {},
            output_format,
        ),
        AppearanceCommand::Set(args) => run_action_with_params(
            args.target,
            ActionKind::AppearanceSet,
            AppearanceSetParams {
                theme: args.theme,
                follow_system_theme: args.follow_system_theme,
                light_theme: args.light_theme,
                dark_theme: args.dark_theme,
            },
            output_format,
        ),
        AppearanceCommand::FontSize(args) => run_action_with_params(
            args.target,
            ActionKind::AppearanceFontSize,
            AppearanceFontSizeParams {
                adjustment: args.adjustment.into(),
                value: args.value,
            },
            output_format,
        ),
        AppearanceCommand::Zoom(args) => run_action_with_params(
            args.target,
            ActionKind::AppearanceZoom,
            AppearanceZoomParams {
                adjustment: args.adjustment.into(),
                value: args.value,
            },
            output_format,
        ),
    }
}

fn run_setting_command(
    command: SettingCommand,
    output_format: OutputFormat,
) -> Result<(), ControlError> {
    match command {
        SettingCommand::List(args) => run_action_with_params(
            args,
            ActionKind::SettingList,
            local_control::SettingListParams::default(),
            output_format,
        ),
        SettingCommand::Get(args) => run_action_with_params(
            args.target,
            ActionKind::SettingGet,
            local_control::SettingGetParams { key: args.key },
            output_format,
        ),
        SettingCommand::Set(args) => run_action_with_params(
            args.target,
            ActionKind::SettingSet,
            SettingSetParams {
                key: args.key,
                value: parse_json_value_or_string(args.value),
            },
            output_format,
        ),
        SettingCommand::Toggle(args) => run_action_with_params(
            args.target,
            ActionKind::SettingToggle,
            SettingToggleParams { key: args.key },
            output_format,
        ),
    }
}

fn run_file_command(command: FileCommand, output_format: OutputFormat) -> Result<(), ControlError> {
    match command {
        FileCommand::List(args) => run_action_with_params(
            args,
            ActionKind::FileList,
            local_control::FileListParams::default(),
            output_format,
        ),
        FileCommand::Open(args) => run_action_with_params(
            args.target,
            ActionKind::FileOpen,
            FileOpenParams {
                path: args.path,
                line: args.line,
                new_window: args.new_window,
            },
            output_format,
        ),
        FileCommand::Write(args) => run_action_with_params(
            args.target,
            ActionKind::FileWrite,
            FileWriteParams {
                path: args.path,
                contents: args.contents,
                create: args.create,
            },
            output_format,
        ),
        FileCommand::Delete(args) => run_action_with_params(
            args.target,
            ActionKind::FileDelete,
            FileDeleteParams {
                path: args.path,
                recursive: args.recursive,
            },
            output_format,
        ),
    }
}

fn run_project_command(
    command: ProjectCommand,
    output_format: OutputFormat,
) -> Result<(), ControlError> {
    match command {
        ProjectCommand::Active(args) => run_action_with_params(
            args,
            ActionKind::ProjectActive,
            local_control::ProjectActiveParams::default(),
            output_format,
        ),
        ProjectCommand::List(args) => run_action_with_params(
            args,
            ActionKind::ProjectList,
            local_control::ProjectListParams::default(),
            output_format,
        ),
    }
}

fn run_drive_command(
    command: DriveCommand,
    output_format: OutputFormat,
) -> Result<(), ControlError> {
    match command {
        DriveCommand::List(args) => run_action_with_params(
            args.target,
            ActionKind::DriveList,
            local_control::DriveListParams {
                object_type: args.object_type.map(Into::into),
            },
            output_format,
        ),
        DriveCommand::Get(args) => run_action_with_params(
            args.target,
            ActionKind::DriveGet,
            local_control::DriveGetParams {
                object_type: args.object_type.into(),
                id: args.id,
            },
            output_format,
        ),
        DriveCommand::Create(args) => run_action_with_params(
            args.target,
            ActionKind::DriveCreate,
            DriveCreateParams {
                object_type: args.object_type.into(),
                name: args.name,
                content: parse_json_value_or_string(args.content),
            },
            output_format,
        ),
        DriveCommand::Update(args) => run_action_with_params(
            args.target,
            ActionKind::DriveUpdate,
            DriveUpdateParams {
                object_type: args.object_type.into(),
                id: args.id,
                content: parse_json_value_or_string(args.content),
            },
            output_format,
        ),
        DriveCommand::Delete(args) => run_action_with_params(
            args.target,
            ActionKind::DriveDelete,
            DriveDeleteParams {
                object_type: args.object_type.into(),
                id: args.id,
            },
            output_format,
        ),
        DriveCommand::Run(args) => run_action_with_params(
            args.target,
            ActionKind::DriveRun,
            DriveRunParams {
                object_type: args.object_type.into(),
                id: args.id,
            },
            output_format,
        ),
        DriveCommand::Insert(args) => run_action_with_params(
            args.target,
            ActionKind::DriveInsert,
            DriveInsertParams {
                object_type: args.object_type.into(),
                id: args.id,
            },
            output_format,
        ),
    }
}

fn run_action_with_params<T: Serialize>(
    args: TargetArgs,
    action: ActionKind,
    params: T,
    output_format: OutputFormat,
) -> Result<(), ControlError> {
    let records = local_control::discovery::list_instances();
    let selector = instance_selector(args);
    let instance = select_instance(&records, &selector)?;
    let request = local_control::RequestEnvelope::new(Action::with_params(action, params)?);
    let response = local_control::client::send_request(&instance, &request)?;
    let local_control::protocol::ControlResponse::Ok { data } = response.response else {
        return Err(ControlError::new(
            ErrorCode::Internal,
            "local-control request failed without an error payload",
        ));
    };
    match output_format {
        OutputFormat::Json => write_json(&data),
        OutputFormat::Ndjson => write_json_line(&data),
        OutputFormat::Pretty | OutputFormat::Text => write_json(&data),
    }
}

fn instance_selector(args: TargetArgs) -> InstanceSelector {
    if let Some(instance_id) = args.instance {
        return InstanceSelector::Id(local_control::discovery::InstanceId(instance_id));
    }
    if let Some(pid) = args.pid {
        return InstanceSelector::Pid(pid);
    }
    InstanceSelector::Active
}

fn generate_completions_to_stdout(shell: Option<Shell>) -> Result<(), ControlError> {
    let shell = shell.or_else(Shell::from_env).ok_or_else(|| {
        ControlError::new(
            ErrorCode::InvalidParams,
            "could not determine shell from environment; provide a shell argument",
        )
    })?;
    let mut cmd = ControlArgs::clap_command();
    let bin_name = crate::binary_name().unwrap_or_else(|| "warpctrl".to_owned());
    generate(shell, &mut cmd, bin_name, &mut std::io::stdout());
    Ok(())
}

#[cfg(test)]
fn generate_completion_string(shell: Shell) -> Result<String, ControlError> {
    let mut cmd = ControlArgs::clap_command();
    let mut output = Vec::new();
    generate(shell, &mut cmd, "warpctrl", &mut output);
    String::from_utf8(output).map_err(|err| {
        ControlError::with_details(
            ErrorCode::Internal,
            "failed to render local-control completions",
            err.to_string(),
        )
    })
}

fn write_control_error(
    error: &ControlError,
    output_format: OutputFormat,
) -> Result<(), ControlError> {
    match output_format {
        OutputFormat::Json => write_json(&ErrorSummary { ok: false, error }),
        OutputFormat::Ndjson => write_json_line(&ErrorSummary { ok: false, error }),
        OutputFormat::Pretty | OutputFormat::Text => {
            eprintln!("error: {}: {}", error.code, error.message);
            if let Some(details) = &error.details {
                eprintln!("details: {details}");
            }
            Ok(())
        }
    }
}

fn write_json(value: &impl Serialize) -> Result<(), ControlError> {
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    serde_json::to_writer_pretty(&mut lock, value).map_err(write_error)?;
    writeln!(&mut lock).map_err(write_error)?;
    Ok(())
}

fn write_json_line(value: &impl Serialize) -> Result<(), ControlError> {
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    serde_json::to_writer(&mut lock, value).map_err(write_error)?;
    writeln!(&mut lock).map_err(write_error)?;
    Ok(())
}

fn write_error(error: impl std::error::Error) -> ControlError {
    ControlError::with_details(
        ErrorCode::Internal,
        "failed to write local-control output",
        error.to_string(),
    )
}

#[cfg(test)]
#[path = "local_control_tests.rs"]
mod tests;
