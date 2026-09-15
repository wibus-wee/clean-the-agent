use std::fmt;

use clean_any::TweakSummary;
use inquire::{Confirm, InquireError, Select};

const NAVIGATION_HELP: &str =
    "Use Up/Down to move, type to filter, Enter to select, or Esc to cancel";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum GuidedAction {
    Scan,
    Clean,
    Tweak,
    Exit,
}

impl fmt::Display for GuidedAction {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Scan => "Scan and review artifacts",
            Self::Clean => "Clean safe artifacts",
            Self::Tweak => "Apply a preference tweak",
            Self::Exit => "Exit",
        })
    }
}

#[derive(Clone, Copy, Debug)]
struct TweakChoice {
    summary: TweakSummary,
}

impl fmt::Display for TweakChoice {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}: {}",
            self.summary.product, self.summary.title
        )
    }
}

pub(crate) fn choose_action() -> Result<Option<GuidedAction>, InquireError> {
    normalize_prompt(
        Select::new(
            "What would you like to do?",
            vec![
                GuidedAction::Scan,
                GuidedAction::Clean,
                GuidedAction::Tweak,
                GuidedAction::Exit,
            ],
        )
        .with_help_message(NAVIGATION_HELP)
        .prompt(),
    )
}

pub(crate) fn choose_tweak(
    summaries: &[TweakSummary],
) -> Result<Option<&'static str>, InquireError> {
    let choices = summaries
        .iter()
        .copied()
        .map(|summary| TweakChoice { summary })
        .collect();
    normalize_prompt(
        Select::new("Which preference would you like to change?", choices)
            .with_help_message(NAVIGATION_HELP)
            .prompt(),
    )
    .map(|choice| choice.map(|choice| choice.summary.id))
}

pub(crate) fn confirm(question: &str) -> Result<bool, InquireError> {
    normalize_prompt(
        Confirm::new(question)
            .with_default(false)
            .with_help_message("Enter confirms the default; Esc cancels")
            .prompt(),
    )
    .map(|answer| answer.unwrap_or(false))
}

fn normalize_prompt<T>(result: Result<T, InquireError>) -> Result<Option<T>, InquireError> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(InquireError::OperationCanceled | InquireError::OperationInterrupted) => Ok(None),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::GuidedAction;

    #[test]
    fn guided_actions_explain_their_effect() {
        assert_eq!(GuidedAction::Scan.to_string(), "Scan and review artifacts");
        assert_eq!(GuidedAction::Clean.to_string(), "Clean safe artifacts");
        assert_eq!(GuidedAction::Tweak.to_string(), "Apply a preference tweak");
        assert_eq!(GuidedAction::Exit.to_string(), "Exit");
    }
}
