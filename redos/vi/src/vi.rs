use alloc::string::{String, ToString};
use redos_abi::rt::abi;

fn str_err<E: ToString>(err: E) -> String {
    err.to_string()
}

enum Mode {
    Normal,
    Insert,
    Command,
    Search,
    SearchBackwards,
}

enum Event {
    Left,
    Right,
    Up,
    Down,
    Home,
    HomeSkipSpace,
    End,
    ScreenLow,
    ScreenMiddle,
    ScreenHigh,
    Search(String),
    SearchBackwards(String),
    CreateLineAfter,
    CreateLineBefore,
    Insert(char),
    Backspace,
    Delete,
    Save,
    Exit,
}

fn with_events<E, F: FnMut(Event) -> Result<(), E>>(
    mode: Mode,
    mut callback: F,
) -> Result<Mode, E> {
    let abi = abi();
    match mode {
        Mode::Normal => {
            let c = abi.key();
            match c {
                // Unknown key
                '\x00' => {}
                // Enter insert mode after cursor
                'a' => {
                    callback(Event::Right)?;
                    return Ok(Mode::Insert);
                }
                // Enter insert mode at end of line
                'A' => {
                    callback(Event::End)?;
                    return Ok(Mode::Insert);
                }
                // Enter insert mode at cursor
                'i' => {
                    return Ok(Mode::Insert);
                }
                // Enter insert mode at start of line
                'I' => {
                    callback(Event::HomeSkipSpace)?;
                    return Ok(Mode::Insert);
                }
                // Create line after and enter insert mode
                'o' => {
                    callback(Event::CreateLineAfter)?;
                    return Ok(Mode::Insert);
                }
                // Create line before and enter insert mode
                'O' => {
                    callback(Event::CreateLineBefore)?;
                    return Ok(Mode::Insert);
                }
                // Left
                'h' | '\u{2190}' => callback(Event::Left)?,
                // Top of screen
                'H' => callback(Event::ScreenHigh)?,
                // Down
                'j' | '\u{2193}' => callback(Event::Down)?,
                // Up
                'k' | '\u{2191}' => callback(Event::Up)?,
                // Right
                'l' | '\u{2192}' => callback(Event::Right)?,
                // Bottom of screen
                'L' => callback(Event::ScreenLow)?,
                // Middle of screen
                'M' => callback(Event::ScreenMiddle)?,
                // Remove character at cursor
                'x' => callback(Event::Delete)?,
                // Remove character before cursor
                'X' => callback(Event::Backspace)?,
                // Go to start of line
                '0' => callback(Event::Home)?,
                // Go to end of line
                '$' => callback(Event::End)?,
                // Go to start of line after whitespace
                '^' => callback(Event::HomeSkipSpace)?,
                // Enter command mode
                ':' => {
                    return Ok(Mode::Command);
                }
                // Enter search mode
                '/' => {
                    return Ok(Mode::Search);
                }
                // Enter search backwards mode
                '?' => {
                    return Ok(Mode::SearchBackwards);
                }
                _ => (),
            }
        }
        Mode::Insert => {
            let c = abi.key();
            match c {
                // Unknown key
                '\x00' => {}
                // Remove character before cursor
                '\x08' => callback(Event::Backspace)?,
                // Exit insert mode
                '\x1B' => {
                    callback(Event::Left)?;
                    return Ok(Mode::Normal);
                }
                // Remove character at cursor
                '\x7F' => callback(Event::Delete)?,
                // Left
                '\u{2190}' => callback(Event::Left)?,
                // Down
                '\u{2193}' => callback(Event::Down)?,
                // Up
                '\u{2191}' => callback(Event::Up)?,
                // Right
                '\u{2192}' => callback(Event::Right)?,
                // Insert character
                _ => callback(Event::Insert(c))?,
            }
        }
        Mode::Command => {
            let line = abi.read_line();
            match line.as_str() {
                "q" => callback(Event::Exit)?,
                "w" => callback(Event::Save)?,
                "x" => {
                    callback(Event::Save)?;
                    callback(Event::Exit)?;
                }
                _ => (),
            }

            return Ok(Mode::Normal);
        }
        Mode::Search => {
            let line = abi.read_line();
            callback(Event::Search(line))?;
            return Ok(Mode::Normal);
        }
        Mode::SearchBackwards => {
            let line = abi.read_line();
            callback(Event::SearchBackwards(line))?;
            return Ok(Mode::Normal);
        }
    }

    Ok(mode)
}

pub(crate) fn vi(args: &[&str]) -> Result<(), String> {
    let abi = abi();

    if args.len() == 0 {
        return Err(str_err("no file provided"));
    }

    for arg in args {
        let fd = abi.open(arg).map_err(str_err)?;
        let mut data = abi.read_to_string(fd).map_err(str_err)?;

        let width = 80;
        let height = 25 - 1; // One line for status line
        let mut scroll = 0;
        let mut cursor_row = 0;
        let mut cursor_col = 0;
        let mut mode = Mode::Normal;
        let mut search_index_opt = None;
        let mut running = true;
        while running {
            abi.text_clear();

            // Ensure there is always a trailing newline
            //TODO: is there a better way to handle this?
            if !data.ends_with('\n') {
                data.push('\n');
            }

            // Start of cursor's row
            let mut cursor_row_start = 0;
            // End of cursor's row
            let mut cursor_row_end = 0;
            // Last col in cursor's row
            let mut cursor_row_col = 0;

            // Actual column cursor index is on (may be less than cursor_col)
            let mut cursor_index_col = 0;
            // Index of cursor
            let mut cursor_index = 0;

            let mut col = 0;
            let mut row = 0;
            for (index, mut c) in data.char_indices() {
                let mut newline = false;
                if c == '\n' {
                    c = ' ';
                    newline = true;
                }

                let mut at_cursor = false;
                if row == cursor_row {
                    if col == cursor_col {
                        // At cursor if on the correct row and column
                        at_cursor = true;
                    } else if newline && col <= cursor_col {
                        // At cursor if on the correct row and there is a newline before the column
                        at_cursor = true;
                    }
                }

                if let Some(search_index) = search_index_opt {
                    if index == search_index {
                        search_index_opt = None;
                        at_cursor = true;
                        cursor_row = row;
                        cursor_col = col;
                    } else {
                        at_cursor = false;
                    }
                }

                if at_cursor {
                    cursor_index = index;
                    cursor_index_col = col;
                }

                // Print if inside bounds
                if row >= scroll && row < scroll + height {
                    if at_cursor {
                        // Highlight if at cursor
                        abi.text_color(0x70);
                        print!("{}", c);
                        abi.text_color(0x07);
                    } else {
                        print!("{}", c);
                    }
                    if newline {
                        // Go to next line if there was a newline
                        println!();
                    }
                }

                // Set newline to true if about to line wrap
                if col + 1 >= width {
                    newline = true;
                }

                if row == cursor_row {
                    if col == 0 {
                        // Store start of row if at column zero
                        cursor_row_start = index;
                    }
                    if newline {
                        // Store end of row if at newline or line wrap
                        cursor_row_col = col;
                        cursor_row_end = index;
                    }
                }

                if newline {
                    // Go to next row and column zero if newline or line wrap
                    row += 1;
                    col = 0;
                } else {
                    // Go to next column
                    col += 1;
                }
            }

            // Fix cursor row if it went too far
            if cursor_row + 1 > row {
                if row > 0 {
                    cursor_row = row - 1;
                } else {
                    cursor_row = 0;
                    cursor_col = 0;
                }
                // Redraw
                continue;
            }

            // Handle lack of trailing newline
            if row == cursor_row {
                cursor_row_col = col;
                cursor_row_end = data.len();
            }

            // Pad to end of screen
            let mut pad_row = row;
            while pad_row < scroll + height {
                println!();
                pad_row += 1;
            }

            match mode {
                Mode::Normal => {
                    print!(
                        "-- NORMAL -- {}, {} in {}, {}",
                        cursor_row, cursor_index_col, row, cursor_row_col
                    );
                }
                Mode::Insert => {
                    print!(
                        "-- INSERT -- {}, {} in {}, {}",
                        cursor_row, cursor_index_col, row, cursor_row_col
                    );
                }
                Mode::Command => {
                    print!(":");
                }
                Mode::Search => {
                    print!("/");
                }
                Mode::SearchBackwards => {
                    print!("?");
                }
            }

            mode = with_events(mode, |event| -> Result<(), String> {
                match event {
                    Event::Left => {
                        if cursor_index_col > 0 {
                            cursor_col = cursor_index_col - 1;
                        }
                    }
                    Event::Right => {
                        if cursor_index_col < cursor_row_col {
                            cursor_col = cursor_index_col + 1;
                        }
                    }
                    Event::Up => {
                        if cursor_row > 0 {
                            cursor_row -= 1;
                            if cursor_row < scroll {
                                scroll = cursor_row;
                            }
                        }
                    }
                    Event::Down => {
                        if cursor_row + 1 < row {
                            cursor_row += 1;
                            if cursor_row >= scroll + height {
                                scroll = cursor_row - (height - 1);
                            }
                        }
                    }
                    Event::Home => {
                        cursor_col = 0;
                    }
                    Event::HomeSkipSpace => {
                        cursor_col = 0;
                        for c in data[cursor_row_start..cursor_row_end].chars() {
                            if c.is_whitespace() {
                                cursor_col += 1;
                            } else {
                                break;
                            }
                        }
                    }
                    Event::End => {
                        cursor_col = cursor_row_col;
                    }
                    Event::ScreenLow => {
                        cursor_row = scroll + height - 1;
                    }
                    Event::ScreenMiddle => {
                        cursor_row = scroll + height / 2;
                    }
                    Event::ScreenHigh => {
                        cursor_row = scroll;
                    }
                    Event::Search(line) => {
                        search_index_opt = match data[cursor_index..].find(&line) {
                            Some(some) => Some(cursor_index + some),
                            None => None,
                        };
                    }
                    Event::SearchBackwards(line) => {
                        search_index_opt = match data[..cursor_index].rfind(&line) {
                            Some(some) => Some(some),
                            None => None,
                        };
                    }
                    Event::CreateLineAfter => {
                        data.insert(cursor_row_end, '\n');
                        cursor_col = 0;
                        cursor_row += 1;
                    }
                    Event::CreateLineBefore => {
                        data.insert(cursor_row_start, '\n');
                        cursor_col = 0;
                    }
                    Event::Insert(c) => {
                        if c.is_control() {
                            match c {
                                '\n' => {
                                    data.insert(cursor_index, '\n');
                                    cursor_row += 1;
                                    cursor_col = 0;
                                }
                                '\r' => {
                                    cursor_col = 0;
                                }
                                _ => (),
                            }
                            //TODO: handle other control characters?
                        } else {
                            data.insert(cursor_index, c);
                            cursor_col = cursor_index_col + 1;
                            if cursor_col >= width {
                                cursor_row += 1;
                                cursor_col = 0;
                            }
                        }
                    }
                    Event::Backspace => {
                        if cursor_index < data.len() {
                            let mut before_opt = None;
                            for (i, _) in data.char_indices() {
                                if i < cursor_index {
                                    before_opt = Some(i);
                                } else {
                                    break;
                                }
                            }
                            if let Some(before) = before_opt {
                                data.remove(before);
                                if cursor_index_col > 0 {
                                    cursor_col = cursor_index_col - 1;
                                } else if cursor_row > 0 {
                                    cursor_row -= 1;
                                    cursor_col = width - 1;
                                }
                            }
                        }
                    }
                    Event::Delete => {
                        if cursor_index < data.len() {
                            data.remove(cursor_index);
                        }
                    }
                    Event::Save => {
                        abi.pwrite(fd, data.as_bytes(), 0).map_err(str_err)?;
                        abi.truncate(fd, data.len()).map_err(str_err)?;
                    }
                    Event::Exit => {
                        running = false;
                    }
                }

                Ok(())
            })?;
        }
    }

    Ok(())
}
