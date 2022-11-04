if args.len() == 0 {
    throw "no file provided"
}

for arg in args {
    let fd = open(arg);
    let data = read(fd);

    let width = 80;
    let height = 25 - 1; // Use last line as status line
    let scroll = 0;
    let cursor_row = 0;
    let cursor_col = 0;
    let command = false;
    let insert = false;
    loop {
        clear();

        // Start of cursor's row
        let cursor_row_start = 0;
        // End of cursor's row
        let cursor_row_end = 0;
        // Last col in cursor's row
        let cursor_row_col = 0;

        // Actual column cursor index is on (may be less than cursor_col)
        let cursor_index_col = 0;
        // Index of cursor
        let cursor_index = 0;

        let index = 0;
        let col = 0;
        let row = 0;
        for c in data.chars() {
            let newline = false;
            if c == '\n' {
                c = ' ';
                newline = true;
            }

            let at_cursor = false;
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
                    highlight(true);
                    putchar(c);
                    highlight(false);
                } else {
                    putchar(c);
                }
                if newline {
                    // Go to next line if there was a newline
                    print();
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

            index += 1;
        }

        // Handle lack of trailing newline
        if row == cursor_row {
            cursor_row_col = col;
            cursor_row_end = index;
        }

        // Pad to end of screen
        let pad_row = row;
        while pad_row + 1 < scroll + height {
            print();
            pad_row += 1;
        }

        if command {
            let line = prompt(":");
            switch line {
                "q" => break,
                "w" =>  write(fd, data),
                "x" =>  {
                    write(fd, data);
                    break;
                },
            }

            command = false;
        } else if insert {
            print(`-- INSERT -- ${cursor_row}, ${cursor_index_col} in ${row}, ${cursor_row_col}`);

            let c = key();
            switch c {
                // Unknown key
                '\x00' => {},
                // Exit insert mode
                '\x1B' => {
                    cursor_col = cursor_index_col;
                    if cursor_col > 0 {
                        cursor_col -= 1;
                    }
                    insert = false;
                },
                // Left
                '\u2190' => if cursor_index_col > 0 {
                    cursor_col = cursor_index_col - 1;
                },
                // Down
                '\u2193' => if cursor_row < row {
                    cursor_row += 1;
                    if cursor_row >= scroll + height {
                        scroll = cursor_row - (height - 1);
                    }
                },
                // Up
                '\u2191' => if cursor_row > 0 {
                    cursor_row -= 1;
                    if cursor_row < scroll {
                        scroll = cursor_row;
                    }
                },
                // Right
                '\u2192' => if cursor_index_col < cursor_row_col {
                    cursor_col = cursor_index_col + 1;
                },
                // Insert character
                _  => {
                    //TODO: string insert
                    let before = data.sub_string(0, cursor_index);
                    let after = data.sub_string(cursor_index);
                    data = before + c + after;
                    cursor_col += 1;
                    if cursor_col >= width {
                        cursor_row += 1;
                        cursor_col = 0;
                    }
                },
            }
        } else {
            print(`-- NORMAL -- ${cursor_row}, ${cursor_index_col} in ${row}, ${cursor_row_col}`);

            let c = key();
            switch c {
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
                // Left
                '\u2190' | 'h' => if cursor_index_col > 0 {
                    cursor_col = cursor_index_col - 1;
                },
                // Down
                '\u2193' | 'j' => if cursor_row < row {
                    cursor_row += 1;
                    if cursor_row >= scroll + height {
                        scroll = cursor_row - (height - 1);
                    }
                },
                // Up
                '\u2191' | 'k' => if cursor_row > 0 {
                    cursor_row -= 1;
                    if cursor_row < scroll {
                        scroll = cursor_row;
                    }
                },
                // Right
                '\u2192' | 'l' => if cursor_index_col + 1 < cursor_row_col {
                    cursor_col = cursor_index_col + 1;
                },
                // Quit
                'q' => break,
                // Remove character at cursor
                'x' => if cursor_index < data.len() {
                    //TODO: string remove
                    let before = data.sub_string(0, cursor_index);
                    let after = data.sub_string(cursor_index + 1);
                    data = before + after;
                },
                // Enter command mode
                ':' => command = true,
            }
        }
    }
}
