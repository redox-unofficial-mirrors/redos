if args.len() == 0 {
    throw "no file provided"
}

for arg in args {
    let data = read(open(arg));

    let width = 80;
    let height = 25 - 1; // Use last line as status line
    let scroll = 0;
    let cursor_row = 0;
    let cursor_col = 0;
    let command = false;
    let insert = false;
    loop {
        clear();

        let cursor_index_col = 0;
        let cursor_index = 0;
        let index = 0;
        let col = 0;
        let row = 0;
        for c in data.chars() {
            let times = 1;
            if c == '\n' {
                if row >= scroll && row < scroll + height {
                    if row == cursor_row && col <= cursor_col {
                        highlight(true);
                        putchar(' ');
                        highlight(false);
                        cursor_index = index;
                        cursor_index_col = col;
                    }
                    print()
                }
                row += 1;
                col = 0;
            } else {
                if col >= width {
                    row += 1;
                    col = 0;
                }
                if row >= scroll && row < scroll + height {
                    if row == cursor_row && col == cursor_col {
                        highlight(true);
                        putchar(c);
                        highlight(false);
                        cursor_index = index;
                        cursor_index_col = col;
                    } else {
                        putchar(c);
                    }
                }
                col += 1;
            }

            index += 1;
        }

        if command {
            // Pad to end of screen
            while row + 1 < scroll + height {
                print();
                row += 1;
            }

            let line = prompt(":");
            switch line {
                "q" => break,
            }

            command = false;
        } else if insert {
            // Pad to end of screen
            while row + 1 < scroll + height {
                print();
                row += 1;
            }

            print("-- INSERT --");

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
                '\u2193' => if cursor_row + 1 < row {
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
                '\u2192' => if cursor_index_col + 1 < width {
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
                '\u2193' | 'j' => if cursor_row + 1 < row {
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
                '\u2192' | 'l' => if cursor_index_col + 1 < width {
                    cursor_col = cursor_index_col + 1;
                },
                // Quit
                'q' => break,
                // Enter command mode
                ':' => command = true,
            }
        }
    }
}
