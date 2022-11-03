if args.len() == 0 {
    throw "no file provided"
}

for arg in args {
    let data = read(open(arg));

    let width = 80;
    let height = 25;
    let scroll = 0;
    let cursor_row = 0;
    let cursor_col = 0;
    let command = false;
    let insert = false;
    loop {
        clear();

        let cursor_index = 0;
        let index = 0;
        let col = 0;
        let row = 0;
        for c in data.chars() {
            let times = 1;
            if c == '\n' {
                c = ' ';
                times = width - col;
            }

            while times > 0 {
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
                    } else {
                        putchar(c);
                    }
                }
                col += 1;
                times -= 1;
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

            height += 1;
            command = false;
        } else if insert {
            let c = key();
            switch c {
                '\x00' => {},
                '\x1B' => insert = false,
                _ => {
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
                'h' => if cursor_col > 0 {
                    cursor_col -= 1;
                },
                'i' => insert = true,
                'j' => if cursor_row < row {
                    cursor_row += 1;
                    if cursor_row >= scroll + height {
                        scroll = cursor_row - (height - 1);
                    }
                },
                'k' => if cursor_row > 0 {
                    cursor_row -= 1;
                    if cursor_row < scroll {
                        scroll = cursor_row;
                    }
                },
                'l' => if cursor_col + 1 < width {
                    cursor_col += 1;
                },
                'q' => break,
                ':' => {
                    height -= 1;
                    command = true;
                },
            }
        }
    }
}
