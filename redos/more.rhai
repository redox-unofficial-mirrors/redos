if args.len() == 0 {
    throw "no file provided"
}

for arg in args {
    let data = read(open(arg));

    let row = 0;
    let line = "";
    for c in data.to_chars() {
        if c == '\n' || line.len() >= 80 {
            if row >= 25 {
                prompt("Press enter for more... ");
                row = 0;
            }
            print(line);
            row += 1;
            line = "";
        } else {
            line += c;
        }
    }
    if line.len() > 0 {
        print(line)
    }
}