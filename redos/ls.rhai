if args.len() == 0 {
    args = ["."];
}

let first = true;
for arg in args {
    if first {
        first = false;
    } else {
        print();
    }
    if args.len() > 1 {
        print(`${arg}:`)
    }
    let entries = readdir(open(arg));
    entries.sort();
    for entry in entries {
        print(`${entry}`)
    }
}