//! Terminal styling: colors, symbols, and progress bars.
//!
//! Colors auto-disable when stdout is not a TTY or `NO_COLOR` is set — the
//! `console` crate handles that detection. Progress bars draw to stderr (via
//! indicatif's default target, which also hides when stderr isn't a TTY) so
//! piped stdout stays clean.

use console::{style, StyledObject};
use indicatif::{ProgressBar, ProgressStyle};
use std::fmt::Display;
use std::time::Duration;

/// Formula name — cyan, bold.
pub fn name<D: Display>(s: D) -> StyledObject<D> {
    style(s).cyan().bold()
}

/// Version string — green.
pub fn version<D: Display>(s: D) -> StyledObject<D> {
    style(s).green()
}

/// Dimmed secondary text.
pub fn dim<D: Display>(s: D) -> StyledObject<D> {
    style(s).dim()
}

/// A URL — blue, underlined.
pub fn url<D: Display>(s: D) -> StyledObject<D> {
    style(s).blue().underlined()
}

/// Leading `==>` step arrow — blue, bold.
pub fn arrow() -> StyledObject<&'static str> {
    style("==>").blue().bold()
}

/// Success check mark — green, bold.
pub fn check() -> StyledObject<&'static str> {
    style("✓").green().bold()
}

/// Neutral bullet for already-present items — dim.
pub fn bullet() -> StyledObject<&'static str> {
    style("•").dim()
}

/// Warning mark — yellow, bold.
pub fn warn() -> StyledObject<&'static str> {
    style("⚠").yellow().bold()
}

/// Failure mark — red, bold.
pub fn fail() -> StyledObject<&'static str> {
    style("✗").red().bold()
}

/// Error label — red, bold. Used for the `error:` prefix.
pub fn error_label() -> StyledObject<&'static str> {
    style("error:").red().bold()
}

const TICKS: &str = "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ ";

/// A byte-progress bar for a bottle download. Falls back to a spinner when the
/// server doesn't report a content length.
pub fn download_bar(total: u64, name: &str, ver: &str) -> ProgressBar {
    let msg = format!(
        "downloading {} {}",
        style(name).cyan().bold(),
        style(ver).green()
    );
    let pb = if total > 0 {
        let pb = ProgressBar::new(total);
        pb.set_style(
            ProgressStyle::with_template(
                "  {spinner:.cyan} {msg} {bar:24.cyan/blue} {bytes:>9}/{total_bytes:<9} {binary_bytes_per_sec:>11}",
            )
            .unwrap()
            .progress_chars("━╾─")
            .tick_chars(TICKS),
        );
        pb
    } else {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::with_template("  {spinner:.cyan} {msg} {bytes}")
                .unwrap()
                .tick_chars(TICKS),
        );
        pb
    };
    pb.set_message(msg);
    pb.enable_steady_tick(Duration::from_millis(80));
    pb
}

/// An indeterminate spinner for a labelled step (extract, relocate, link).
pub fn step_spinner(msg: String) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("  {spinner:.cyan} {msg}")
            .unwrap()
            .tick_chars(TICKS),
    );
    pb.set_message(msg);
    pb.enable_steady_tick(Duration::from_millis(80));
    pb
}
