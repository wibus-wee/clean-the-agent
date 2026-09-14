use std::error::Error;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};
use clean_any::model::{CleanupPlan, ScanReport};
use clean_any::providers::OrcaProvider;
use clean_any::{Engine, HomeScope, ScanContext, ScopeKind};
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

#[derive(Serialize)]
struct CleanOutput<'a> {
    scan: &'a ScanReport,
    plan: &'a CleanupPlan,
    applied: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<&'a clean_any::ApplyReport>,
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
    if matches!(cli.command, Command::Providers) {
        for id in engine.provider_ids() {
            println!("{id}");
        }
        return Ok(ExitCode::SUCCESS);
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
    let scan = engine.scan(&context, &cli.provider)?;

    match cli.command {
        Command::Scan(args) => {
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
        Command::Providers => unreachable!("handled before scanning"),
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
