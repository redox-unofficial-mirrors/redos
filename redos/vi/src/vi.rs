use alloc::string::{String, ToString};
use redos_abi::{error::{self, Error}, rt::abi};

fn str_err<E: ToString>(err: E) -> String {
    err.to_string()
}

pub(crate) fn vi(args: &[&str]) -> Result<(), String> {
    let abi = abi();

    if args.len() == 0 {
        return Err(str_err("no file provided"))
    }

    for arg in args {
        let fd = abi.open(arg).map_err(str_err)?;
        let mut data = abi.read_to_string(fd).map_err(str_err)?;

        let width = 80;
        let height = 25 - 1; // One line for status line
        let mut scroll = 0;
        let mut cursor_row = 0;
        let mut cursor_col = 0;
        let mut command = false;
        let mut insert = false;
        loop {
            abi.text_clear();

            // Ensure there is always a trailing newline
            //TODO: is there a better way to handle this?
            if ! data.ends_with('\n') {
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

            if command {
                let prompt = ":";
                print!("{}", prompt);
                let line = abi.read_line();
                for c in prompt.chars().chain(line.chars()) {
                    print!("\x08 \x08");
                }

                match line.as_str() {
                    "q" => break,
                    "w" =>  {
                        abi.pwrite(fd, data.as_bytes(), 0).map_err(str_err)?;
                        abi.truncate(fd, data.len()).map_err(str_err)?;
                    },
                    "x" =>  {
                        abi.pwrite(fd, data.as_bytes(), 0).map_err(str_err)?;
                        abi.truncate(fd, data.len()).map_err(str_err)?;
                        break;
                    },
                    _ => (),
                }

                command = false;
            } else if insert {
                println!("-- INSERT -- {}, {} ({}) in {}, {}", cursor_row, cursor_index_col, cursor_col, row, cursor_row_col);

                let c = abi.key();
                match c {
                    // Unknown key
                    '\x00' => {},
                    // Remove character before cursor
                    '\x08' => if cursor_index < data.len() {
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
                                cursor_col = width - 1;
                                cursor_row -= 1;
                            }
                        }
                    },
                    // Exit insert mode
                    '\x1B' => {
                        cursor_col = cursor_index_col;
                        if cursor_col > 0 {
                            cursor_col -= 1;
                        }
                        insert = false;
                    },
                    // Remove character at cursor
                    '\x7F' => if cursor_index < data.len() {
                        data.remove(cursor_index);
                    },
                    // Left
                    '\u{2190}' => if cursor_index_col > 0 {
                        cursor_col = cursor_index_col - 1;
                    },
                    // Down
                    '\u{2193}' => if cursor_row + 1 < row {
                        cursor_row += 1;
                        if cursor_row >= scroll + height {
                            scroll = cursor_row - (height - 1);
                        }
                    },
                    // Up
                    '\u{2191}' => if cursor_row > 0 {
                        cursor_row -= 1;
                        if cursor_row < scroll {
                            scroll = cursor_row;
                        }
                    },
                    // Right
                    '\u{2192}' => if cursor_index_col < cursor_row_col {
                        cursor_col = cursor_index_col + 1;
                    },
                    // New line
                    '\n' | '\r' => {
                        data.insert(cursor_index, '\n');
                        cursor_row += 1;
                        cursor_col = 0;
                    },
                    // Insert character
                    _  => {
                        if ! c.is_control() {
                            data.insert(cursor_index, c);
                            cursor_col = cursor_index_col + 1;
                            if cursor_col >= width {
                                cursor_row += 1;
                                cursor_col = 0;
                            }
                        }
                    },
                }
            } else {
                println!("-- NORMAL -- {}, {} in {}, {}", cursor_row, cursor_index_col, row, cursor_row_col);

                let c = abi.key();
                match c {
                    // Unknown key
                    '\x00' => {},
                    // Enter insert mode after cursor
                    'a' => {
                        cursor_col = cursor_index_col;
                        if cursor_col + 1 < width {
                            cursor_col += 1;
                        }
                        insert = true;
                    },
                    // Enter insert mode at end of line
                    'A' => {
                        cursor_col = width - 1;
                        insert = true;
                    }
                    // Enter insert mode at cursor
                    'i' => insert = true,
                    // Enter insert mode at start of line
                    'I' => {
                        cursor_col = 0;
                        insert = true;
                    }
                    // Create line after and enter insert mode
                    'o' => {
                        data.insert(cursor_row_end, '\n');
                        cursor_col = 0;
                        cursor_row += 1;
                        insert = true;
                    },
                    // Create line before and enter insert mode
                    'O' => {
                        data.insert(cursor_row_start, '\n');
                        cursor_col = 0;
                        insert = true;
                    },
                    // Left
                    '\u{2190}' | 'h' => if cursor_index_col > 0 {
                        cursor_col = cursor_index_col - 1;
                    },
                    // Down
                    '\u{2193}' | 'j' => if cursor_row + 1 < row {
                        cursor_row += 1;
                        if cursor_row >= scroll + height {
                            scroll = cursor_row - (height - 1);
                        }
                    },
                    // Up
                    '\u{2191}' | 'k' => if cursor_row > 0 {
                        cursor_row -= 1;
                        if cursor_row < scroll {
                            scroll = cursor_row;
                        }
                    },
                    // Right
                    '\u{2192}' | 'l' => if cursor_index_col + 1 < cursor_row_col {
                        cursor_col = cursor_index_col + 1;
                    },
                    // Quit
                    'q' => break,
                    // Remove character at cursor
                    'x' => if cursor_index < data.len() {
                        data.remove(cursor_index);
                    },
                    // Remove character before cursor
                    'X' => if cursor_index < data.len() {
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
                                cursor_col = width - 1;
                                cursor_row -= 1;
                            }
                        }
                    },
                    // Enter command mode
                    ':' => command = true,
                    _ => (),
                }
            }
        }
    }

    Ok(())
}
