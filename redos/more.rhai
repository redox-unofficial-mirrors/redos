if args.len() == 0 {
    throw "no file provided"
}

for arg in args {
    let data = read(open(arg));

    let col = 0;
    let row = 0;
    for c in data.to_chars() {
        putchar(c);
        if c == '\n' {
            col = 80;
        } else {
            col += 1;
        }
        if col >= 80 {
            col = 0;
            row += 1;
            if row >= 24 {
                prompt("Press enter for more... ");
                row = 0;
            }
        }
    }
}
