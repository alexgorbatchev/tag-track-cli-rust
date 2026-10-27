use crate::Result;
use cli_command_kit::Mode;
use std::io::Write;
pub fn print_outcome(outcome: &crate::Outcome, mode: Mode, output: &mut impl Write) -> Result<()> {
    let metadata = &outcome.metadata;
    if mode == Mode::Agent {
        writeln!(
            output,
            "title: {}\nartist: {}\nalbum: {}\ndate: {}\nyear: {}\ngenre: {}\ncover_art: {}\nsource: {}\ndry_run: {}\noutput: {}",
            cli_command_kit::display_message(&metadata.title),
            cli_command_kit::display_message(&metadata.artist),
            cli_command_kit::display_message(&metadata.album),
            cli_command_kit::display_message(&metadata.release_date),
            metadata.release_year(),
            cli_command_kit::display_message(&metadata.genre),
            cli_command_kit::display_message(&metadata.cover_url),
            cli_command_kit::display_message(&metadata.provider),
            outcome.dry_run,
            cli_command_kit::display_path(&outcome.path)
        )?;
    } else if outcome.dry_run {
        writeln!(
            output,
            "Preview: {} - {}\nalbum: {}\ndate: {}\ngenre: {}\ncover_art: {}\nsource: {}\n[DRY RUN] {}",
            cli_command_kit::display_message(&metadata.artist),
            cli_command_kit::display_message(&metadata.title),
            cli_command_kit::display_message(&metadata.album),
            cli_command_kit::display_message(&metadata.release_date),
            cli_command_kit::display_message(&metadata.genre),
            cli_command_kit::display_message(&metadata.cover_url),
            cli_command_kit::display_message(&metadata.provider),
            cli_command_kit::display_path(&outcome.path)
        )?;
    } else {
        writeln!(
            output,
            "Matched: {} - {}\nDONE: {}",
            cli_command_kit::display_message(&metadata.artist),
            cli_command_kit::display_message(&metadata.title),
            cli_command_kit::display_path(&outcome.path)
        )?;
    }
    Ok(())
}
pub fn print_inspection(
    info: &audio_metadata::inspect::Inspection,
    mode: Mode,
    output: &mut impl Write,
) -> Result<()> {
    let label = if mode == Mode::Agent { "file" } else { "File" };
    writeln!(
        output,
        "{label}: {}\nformat: {}\ncodec: {}\nduration: {:.3} s\nbitrate: {} kbps\nsample_rate: {} Hz\nchannels: {}",
        cli_command_kit::display_path(&info.path),
        info.format,
        info.codec,
        info.duration_seconds,
        info.bit_rate_kbps,
        info.sample_rate_hz,
        info.channels
    )?;
    for (key, value) in [
        ("title", &info.title),
        ("artist", &info.artist),
        ("album", &info.album),
        ("date", &info.date),
        ("genre", &info.genre),
        ("comment", &info.comment),
    ] {
        if !value.is_empty() {
            writeln!(output, "{key}: {}", cli_command_kit::display_message(value))?;
        }
    }
    writeln!(output, "has_artwork: {}", info.has_artwork)?;
    if info.has_artwork {
        writeln!(
            output,
            "artwork_dimensions: {}x{}\nartwork_square: {}",
            info.artwork_width, info.artwork_height, info.is_square_artwork
        )?;
    }
    Ok(())
}
