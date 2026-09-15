mod cli_interactive;
mod cli_output;

use std::error::Error;
use std::io::{self, IsTerminal};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};
use clean_any::model::CleanupPlan;
use clean_any::providers::OrcaProvider;
use clean_any::tweaks::DisableCodexPetShortcut;
use clean_any::{Engine, HomeScope, ScanContext, ScopeKind, TweakEngine, TweakStatus};

use crate::cli_interactive::{GuidedAction, choose_action, choose_tweak, confirm};
use crate::cli_output::{CommandOutput, OutputOptions, emit};

#[derive(Debug, Parser)]
#[command(name = "clean-the-agent", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Scan only these providers. May be repeated.
    #[arg(long, global = true, value_name = "ID")]
    provider: Vec<String>,

    /// Use an alternate home directory and ignore host environment paths.
    #[arg(long, global = true, value_name = "PATH")]
    home: Option<PathBuf>,

    /// Add a known Orca data directory. May be repeated.
    #[arg(long, global = true, value_name = "PATH")]
    orca_data_dir: Vec<PathBuf>,

    /// Add an Orca workspace root to inspect for stale trash. May be repeated.
    #[arg(long, global = true, value_name = "PATH")]
    workspace_root: Vec<PathBuf>,

    /// Add a mounted or directly accessible WSL home to inspect. May be repeated.
    #[arg(long, global = true, value_name = "PATH")]
    wsl_home: Vec<PathBuf>,

    /// Add a mounted or directly accessible SSH remote home. May be repeated.
    #[arg(long, global = true, value_name = "PATH")]
    remote_home: Vec<PathBuf>,

    /// Treat age-based logs and temporary state older than this many days as stale.
    #[arg(long, global = true, default_value_t = 30, value_name = "DAYS")]
    stale_days: u64,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Open a guided terminal menu. This is also the default with no command.
    Interactive(InteractiveArgs),
    /// Detect and explain provider-owned artifacts and mutations.
    Scan(OutputArgs),
    /// Build a cleanup plan, then optionally apply it.
    Clean(CleanArgs),
    /// List compiled-in providers.
    Providers(OutputArgs),
    /// List explicitly selected preference tweaks.
    Tweaks(OutputArgs),
    /// Inspect and optionally apply one preference tweak.
    Tweak(TweakArgs),
}

#[derive(Clone, Debug, Default, Args)]
struct OutputArgs {
    /// Emit machine-readable JSON.
    #[arg(long)]
    json: bool,

    /// Show technical details and detection evidence.
    #[arg(long, short)]
    verbose: bool,
}

#[derive(Debug, Args)]
struct InteractiveArgs {
    /// Show technical details and detection evidence.
    #[arg(long, short)]
    verbose: bool,
}

#[derive(Debug, Args)]
struct CleanArgs {
    /// Apply the displayed plan. Without this flag, clean is a dry run.
    #[arg(long)]
    apply: bool,

    /// Skip the interactive confirmation prompt.
    #[arg(long, requires = "apply")]
    yes: bool,

    /// Include cleanup actions that require manual review.
    #[arg(long)]
    include_review: bool,

    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Debug, Args)]
struct TweakArgs {
    /// Stable tweak identifier, such as codex.disable-pet-shortcut.
    id: String,

    /// Apply the displayed tweak. Without this flag, the command is a dry run.
    #[arg(long)]
    apply: bool,

    /// Skip the interactive confirmation prompt.
    #[arg(long, requires = "apply")]
    yes: bool,

    #[command(flatten)]
    output: OutputArgs,
}

impl OutputArgs {
    fn options(&self) -> OutputOptions {
        OutputOptions {
            json: self.json,
            verbose: self.verbose,
        }
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<ExitCode, Box<dyn Error>> {
    let cli = Cli::parse();
    let engine = Engine::new(vec![Box::new(OrcaProvider)]);
    let tweak_engine = TweakEngine::new(vec![Box::new(DisableCodexPetShortcut)]);
    match cli.command.as_ref() {
        Some(Command::Providers(args)) => {
            let providers = engine.provider_ids();
            emit(
                CommandOutput::Providers {
                    providers: &providers,
                },
                args.options(),
            )?;
            return Ok(ExitCode::SUCCESS);
        }
        Some(Command::Tweaks(args)) => {
            let summaries = tweak_engine.summaries();
            emit(
                CommandOutput::Tweaks {
                    summaries: &summaries,
                },
                args.options(),
            )?;
            return Ok(ExitCode::SUCCESS);
        }
        _ => {}
    }

    let mut context =
        ScanContext::from_environment(cli.home, cli.orca_data_dir, cli.workspace_root)?;
    context.stale_after_days = cli.stale_days;
    context
        .additional_homes
        .extend(cli.wsl_home.into_iter().map(|home| HomeScope {
            home,
            kind: ScopeKind::Wsl,
        }));
    context
        .additional_homes
        .extend(cli.remote_home.into_iter().map(|home| HomeScope {
            home,
            kind: ScopeKind::SshRemote,
        }));

    match cli.command {
        Some(Command::Scan(args)) => run_scan(&args, &engine, &context, &cli.provider),
        Some(Command::Clean(args)) => run_clean(&args, &engine, &context, &cli.provider),
        Some(Command::Tweak(args)) => run_tweak(&args, &tweak_engine, &context),
        Some(Command::Interactive(args)) => run_interactive(
            &OutputArgs {
                json: false,
                verbose: args.verbose,
            },
            &engine,
            &tweak_engine,
            &context,
            &cli.provider,
        ),
        None => run_interactive(
            &OutputArgs::default(),
            &engine,
            &tweak_engine,
            &context,
            &cli.provider,
        ),
        Some(Command::Providers(_) | Command::Tweaks(_)) => {
            unreachable!("handled before context setup")
        }
    }
}

fn run_interactive(
    output: &OutputArgs,
    engine: &Engine,
    tweak_engine: &TweakEngine,
    context: &ScanContext,
    providers: &[String],
) -> Result<ExitCode, Box<dyn Error>> {
    require_guided_terminal()?;
    match choose_action()? {
        Some(GuidedAction::Scan) => run_scan(output, engine, context, providers),
        Some(GuidedAction::Clean) => run_clean(
            &CleanArgs {
                apply: true,
                yes: false,
                include_review: false,
                output: output.clone(),
            },
            engine,
            context,
            providers,
        ),
        Some(GuidedAction::Tweak) => {
            let summaries = tweak_engine.summaries();
            let Some(id) = choose_tweak(&summaries)? else {
                emit(
                    CommandOutput::Cancelled {
                        operation: "Tweak selection",
                    },
                    output.options(),
                )?;
                return Ok(ExitCode::SUCCESS);
            };
            run_tweak(
                &TweakArgs {
                    id: id.to_owned(),
                    apply: true,
                    yes: false,
                    output: output.clone(),
                },
                tweak_engine,
                context,
            )
        }
        Some(GuidedAction::Exit) | None => {
            emit(
                CommandOutput::Cancelled {
                    operation: "Interactive session",
                },
                output.options(),
            )?;
            Ok(ExitCode::SUCCESS)
        }
    }
}

fn run_scan(
    args: &OutputArgs,
    engine: &Engine,
    context: &ScanContext,
    providers: &[String],
) -> Result<ExitCode, Box<dyn Error>> {
    let scan = engine.scan(context, providers)?;
    emit(
        CommandOutput::Scan {
            report: &scan,
            home: &context.home,
        },
        args.options(),
    )?;
    Ok(ExitCode::SUCCESS)
}

fn run_clean(
    args: &CleanArgs,
    engine: &Engine,
    context: &ScanContext,
    providers: &[String],
) -> Result<ExitCode, Box<dyn Error>> {
    let scan = engine.scan(context, providers)?;
    let plan = Engine::plan(&scan, args.include_review);
    if !args.apply || plan.actions.is_empty() {
        emit(
            CommandOutput::CleanPreview {
                scan: &scan,
                plan: &plan,
                home: &context.home,
                awaiting_confirmation: false,
            },
            args.output.options(),
        )?;
        return Ok(ExitCode::SUCCESS);
    }

    if !args.yes {
        require_apply_prompt(&args.output)?;
        emit(
            CommandOutput::CleanPreview {
                scan: &scan,
                plan: &plan,
                home: &context.home,
                awaiting_confirmation: true,
            },
            args.output.options(),
        )?;
        if !confirm("Apply this cleanup plan?")? {
            emit(
                CommandOutput::Cancelled {
                    operation: "Cleanup",
                },
                args.output.options(),
            )?;
            return Ok(ExitCode::SUCCESS);
        }
    }

    let result = Engine::apply(&plan);
    let failed = result.has_failures();
    emit(
        CommandOutput::CleanApplied {
            scan: &scan,
            plan: &plan,
            result: &result,
            home: &context.home,
        },
        args.output.options(),
    )?;
    Ok(if failed {
        ExitCode::from(2)
    } else {
        ExitCode::SUCCESS
    })
}

fn run_tweak(
    args: &TweakArgs,
    tweak_engine: &TweakEngine,
    context: &ScanContext,
) -> Result<ExitCode, Box<dyn Error>> {
    let report = tweak_engine.inspect(&args.id, context)?;
    if !args.apply {
        emit(
            CommandOutput::TweakPreview {
                report: &report,
                home: &context.home,
                awaiting_confirmation: false,
            },
            args.output.options(),
        )?;
        return Ok(ExitCode::SUCCESS);
    }

    let Some(action) = report.action.clone() else {
        emit(
            CommandOutput::TweakPreview {
                report: &report,
                home: &context.home,
                awaiting_confirmation: false,
            },
            args.output.options(),
        )?;
        return Ok(if report.status == TweakStatus::Blocked {
            ExitCode::from(2)
        } else {
            ExitCode::SUCCESS
        });
    };
    if !args.yes {
        require_apply_prompt(&args.output)?;
        emit(
            CommandOutput::TweakPreview {
                report: &report,
                home: &context.home,
                awaiting_confirmation: true,
            },
            args.output.options(),
        )?;
        if !confirm("Apply this tweak?")? {
            emit(
                CommandOutput::Cancelled { operation: "Tweak" },
                args.output.options(),
            )?;
            return Ok(ExitCode::SUCCESS);
        }
    }
    let result = Engine::apply(&CleanupPlan {
        actions: vec![action],
        excluded_review_findings: 0,
        reclaimable_bytes: 0,
    });
    let failed = result.has_failures();
    emit(
        CommandOutput::TweakApplied {
            report: &report,
            result: &result,
            home: &context.home,
        },
        args.output.options(),
    )?;
    Ok(if failed {
        ExitCode::from(2)
    } else {
        ExitCode::SUCCESS
    })
}

fn require_guided_terminal() -> Result<(), Box<dyn Error>> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(
            "interactive mode requires a terminal; choose scan, clean, tweaks, or tweak for non-interactive use"
                .into(),
        );
    }
    Ok(())
}

fn require_apply_prompt(output: &OutputArgs) -> Result<(), Box<dyn Error>> {
    if output.json {
        return Err("--json --apply requires --yes because JSON output cannot prompt".into());
    }
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err("--apply requires an interactive terminal or --yes".into());
    }
    Ok(())
}
