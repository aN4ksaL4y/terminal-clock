// src/main.rs
use std::{
    io::{self, Write},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyModifiers},
    execute, // Used for execute! macro
    style::{PrintStyledContent, Stylize, Color}, // Import Color enum for specific colors
    terminal::{self, Clear, ClearType}, // `self` is needed for `terminal::size()`
    QueueableCommand, // Used for stdout.queue()
};
use std::io::Result; // Correctly import Result from std::io
use figlet_rs::FIGfont;
use time_format::{now, strftime_local};
use std::str; // Import the str module for from_utf8

// Embed the colossal.flf font file directly into the binary
// The path is relative to the current source file (src/main.rs)
static COLOSSAL_FONT_BYTES: &[u8] = include_bytes!("../resources/colossal.flf");

fn main() -> Result<()> {
    let mut stdout = io::stdout();

    // 1. Enable raw mode and hide the cursor for a clean display [1, 2]
    terminal::enable_raw_mode()?;
    execute!(stdout, cursor::Hide)?;

    // 2. Set up Ctrl+C handling in a separate thread [3]
    // This allows for graceful exit even when raw mode intercepts signals.
    let running = Arc::new(AtomicBool::new(true));
    let r_clone = running.clone();
    thread::spawn(move |

| -> Result<()> {
        loop {
            // Poll for events every 100ms to remain responsive [3]
            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key_event) = event::read()? {
                    // Check for Ctrl+C (KeyCode::Char('c') with KeyModifiers::CONTROL)
                    if key_event.code == KeyCode::Char('c') && key_event.modifiers.contains(KeyModifiers::CONTROL) {
                        r_clone.store(false, Ordering::SeqCst); // Signal the main thread to stop
                        break;
                    }
                }
            }
            // Also check if the main thread has already signaled to stop (e.g., on error)
            if!r_clone.load(Ordering::SeqCst) {
                break;
            }
        }
        Ok(())
    });

    // 3. Load the custom "Colossal" FIGlet font from the embedded bytes
    // Convert the byte slice to a string slice, assuming valid UTF-8 [4]
    let font_content = str::from_utf8(COLOSSAL_FONT_BYTES).expect("Colossal font file is not valid UTF-8");
    let standard_font = FIGfont::from_content(font_content).unwrap(); // Use from_content [5]

    // Initialize time tracking for consistent updates
    let mut last_update_time = Instant::now();
    let update_interval = Duration::from_secs(1); // Update every 1 second

    // Main application loop: continues until Ctrl+C is detected or an error occurs
    while running.load(Ordering::SeqCst) {
        // 4. Get the current time and format it as HH:MM:SS [6]
        let current_timestamp = now().unwrap();
        let time_string = strftime_local("%H:%M:%S", current_timestamp).unwrap();

        // 5. Generate the large ASCII art representation of the time [7, 8]
        let figure = standard_font.convert(&time_string);
        let ascii_art_figure = figure.expect("Could not convert time to ASCII art");
        let ascii_art_string = ascii_art_figure.to_string(); // Convert FIGure to String [9]

        // Calculate the dimensions of the generated ASCII art
        let ascii_art_lines: Vec<&str> = ascii_art_string.lines().collect();
        let ascii_art_height = ascii_art_lines.len() as u16;
        let ascii_art_width = ascii_art_lines.iter().map(|line| line.len()).max().unwrap_or(0) as u16;

        // 6. Get the current terminal dimensions [10, 11, 12]
        let (terminal_width, terminal_height) = terminal::size()?;

        // 7. Calculate the top-left coordinates to center the ASCII art
        let start_col = terminal_width.saturating_sub(ascii_art_width) / 2;
        let start_row = terminal_height.saturating_sub(ascii_art_height) / 2;

        // 8. Queue terminal commands for efficient, flicker-free updates
        // Clear the entire screen
        stdout.queue(Clear(ClearType::All))?;

        // Print each line of the ASCII art, moving the cursor for each line
        let mut current_print_row = start_row;
        for line in ascii_art_lines {
            stdout.queue(cursor::MoveTo(start_col, current_print_row))?;
            // Print in green color
            stdout.queue(PrintStyledContent(line.to_string().with(Color::Green)))?;
            current_print_row += 1;
        }

        // 9. Flush all queued commands to the terminal at once
        stdout.flush()?;

        // 10. Control the update rate to approximately 1 second [13, 14]
        let elapsed = last_update_time.elapsed();
        if elapsed < update_interval {
            thread::sleep(update_interval - elapsed); // Sleep for the remaining time
        }
        last_update_time = Instant::now(); // Reset the timer for the next update
    }

    // 11. Cleanup: Restore terminal state before exiting [15, 16]
    // This is crucial to prevent a "corrupted" terminal after the application closes.
    execute!(stdout, cursor::Show)?; // Make the cursor visible again [1, 2]
    terminal::disable_raw_mode()?; // Disable raw mode [17, 18]
    execute!(stdout, Clear(ClearType::All))?; // Clear the screen one last time [10, 11, 12]

    Ok(())
}