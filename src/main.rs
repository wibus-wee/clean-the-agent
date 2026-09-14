use std::error::Error;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};
use clean_any::model::{CleanupPlan, ScanReport};
use clean_any::providers::OrcaProvider;
use clean_any::tweaks::DisableCodexPetShortcut;
use clean_any::{
    ApplyReport, Engine, HomeScope, ScanContext, ScopeKind, TweakEngine, TweakReport, TweakStatus,
};
use serde::Serialize;

#[derive(Debug, Parser)]
#[command(name = "clean-any", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,

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
    /// Detect and explain provider-owned artifacts and mutations.
    Scan(OutputArgs),
    /// Build a cleanup plan, then optionally apply it.
    Clean(CleanArgs),
    /// List compiled-in providers.
    Providers,
    /// List explicitly selected preference tweaks.
    Tweaks(OutputArgs),
    /// Inspect and optionally apply one preference tweak.
    Tweak(TweakArgs),
}

#[derive(Clone, Debug, Args)]
struct OutputArgs {
    /// Emit machine-readable JSON.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct CleanArgs {
    /// Apply the displayed plan. Without this flag, clean is a dry run.
    #[arg(long)]
    apply: bool,

    /// Confirm an apply operation.
    #[arg(long, requires = "apply")]
    yes: bool,

    /// Include attributed worktrees after the automatic checks pass.
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

    /// Confirm an apply operation.
    #[arg(long, requires = "apply")]
    yes: bool,

    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Serialize)]
struct CleanOutput<'a> {
    scan: &'a ScanReport,
    plan: &'a CleanupPlan,
    applied: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<&'a clean_any::ApplyReport>,
}

#[derive(Serialize)]
struct TweakOutput<'a> {
    report: &'a TweakReport,
    applied: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<&'a ApplyReport>,
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
    match &cli.command {
        Command::Providers => {
            for id in engine.provider_ids() {
                println!("{id}");
            }
            return Ok(ExitCode::SUCCESS);
        }
        Command::Tweaks(args) => {
            let summaries = tweak_engine.summaries();
            if args.json {
                println!("{}", serde_json::to_string_pretty(&summaries)?);
            } else {
                for summary in summaries {
                    println!("{}  {} — {}", summary.id, summary.product, summary.title);
                    println!("  {}", summary.description);
                }
            }
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
        Command::Scan(args) => {
            let scan = engine.scan(&context, &cli.provider)?;
            if args.json {
                println!("{}", serde_json::to_string_pretty(&scan)?);
            } else {
                print_scan(&scan);
            }
            Ok(ExitCode::SUCCESS)
        }
        Command::Clean(args) => {
            if args.apply && !args.yes {
                return Err("--apply requires --yes after reviewing the dry-run plan".into());
            }
            let scan = engine.scan(&context, &cli.provider)?;
            let plan = Engine::plan(&scan, args.include_review);
            if !args.apply {
                if args.output.json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&CleanOutput {
                            scan: &scan,
                            plan: &plan,
                            applied: false,
                            result: None,
                        })?
                    );
                } else {
                    print_plan(&plan);
                    println!("Dry run only. Re-run with --apply --yes to execute this plan.");
                }
                return Ok(ExitCode::SUCCESS);
            }

            let result = Engine::apply(&plan);
            let failed = result.has_failures();
            if args.output.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&CleanOutput {
                        scan: &scan,
                        plan: &plan,
                        applied: true,
                        result: Some(&result),
                    })?
                );
            } else {
                print_apply(&result);
            }
            Ok(if failed {
                ExitCode::from(2)
            } else {
                ExitCode::SUCCESS
            })
        }
        Command::Tweak(args) => run_tweak(&args, &tweak_engine, &context),
        Command::Providers | Command::Tweaks(_) => unreachable!("handled before context setup"),
    }
}

fn run_tweak(
    args: &TweakArgs,
    tweak_engine: &TweakEngine,
    context: &ScanContext,
) -> Result<ExitCode, Box<dyn Error>> {
    if args.apply && !args.yes {
        return Err("--apply requires --yes after reviewing the dry-run tweak".into());
    }
    let report = tweak_engine.inspect(&args.id, context)?;
    if !args.apply {
        print_tweak_output(&report, args.output.json, false, None)?;
        if !args.output.json && report.status == TweakStatus::NeedsChange {
            println!("Dry run only. Re-run with --apply --yes to apply this tweak.");
        }
        return Ok(ExitCode::SUCCESS);
    }

    let Some(action) = report.action.clone() else {
        print_tweak_output(&report, args.output.json, false, None)?;
        return Ok(if report.status == TweakStatus::Blocked {
            ExitCode::from(2)
        } else {
            ExitCode::SUCCESS
        });
    };
    let result = Engine::apply(&CleanupPlan {
        actions: vec![action],
        excluded_review_findings: 0,
        reclaimable_bytes: 0,
    });
    let failed = result.has_failures();
    print_tweak_output(&report, args.output.json, true, Some(&result))?;
    if !args.output.json && !failed && report.restart_required {
        println!("Restart Codex for the shortcut change to take effect.");
    }
    Ok(if failed {
        ExitCode::from(2)
    } else {
        ExitCode::SUCCESS
    })
}

fn print_tweak_output(
    report: &TweakReport,
    json: bool,
    applied: bool,
    result: Option<&ApplyReport>,
) -> Result<(), serde_json::Error> {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&TweakOutput {
                report,
                applied,
                result,
            })?
        );
    } else if let Some(result) = result {
        print_apply(result);
    } else {
        print_tweak(report);
    }
    Ok(())
}

fn print_tweak(report: &TweakReport) {
    println!("Tweak: {}", report.id);
    println!("Status: {:?}", report.status);
    println!("Target: {}", report.path.display());
    println!("  {}", report.description);
    println!("  {}", report.detail);
    if report.restart_required {
        println!("  Codex restart required after applying.");
    }
}

fn print_scan(report: &ScanReport) {
    if report.findings.is_empty() {
        println!("No supported artifacts or mutations found.");
    }
    for finding in &report.findings {
        println!(
            "[{:?}/{:?}] {:?}: {}",
            finding.safety,
            finding.scope,
            finding.kind,
            finding.path.display()
        );
        println!("  {}", finding.description);
        println!("  Evidence: {}", finding.evidence);
        if finding.reclaimable_bytes > 0 {
            println!("  Size: {}", human_bytes(finding.reclaimable_bytes));
        }
    }
    for warning in &report.warnings {
        eprintln!("warning: {warning}");
    }
    println!(
        "{} finding(s), {} warning(s)",
        report.findings.len(),
        report.warnings.len()
    );
}

fn print_plan(plan: &CleanupPlan) {
    println!(
        "Plan: {} action(s), reclaiming up to {}",
        plan.actions.len(),
        human_bytes(plan.reclaimable_bytes)
    );
    for action in &plan.actions {
        println!("- {:?}: {}", action.safety, action.path.display());
        println!("  {}", action.description);
    }
    if plan.excluded_review_findings > 0 {
        println!(
            "{} review-required action(s) excluded; inspect them with scan and opt in with --include-review.",
            plan.excluded_review_findings
        );
    }
}

fn print_apply(report: &clean_any::ApplyReport) {
    for result in &report.results {
        println!(
            "[{:?}] {}: {}",
            result.status,
            result.path.display(),
            result.detail
        );
    }
}

fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut divisor = 1_u64;
    let mut unit = 0;
    while bytes / divisor >= 1024 && unit < UNITS.len() - 1 {
        divisor *= 1024;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} {}", UNITS[unit])
    } else {
        let whole = bytes / divisor;
        let tenths = (bytes % divisor) * 10 / divisor;
        format!("{whole}.{tenths} {}", UNITS[unit])
    }
}
