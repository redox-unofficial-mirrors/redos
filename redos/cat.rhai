if args.len() == 0 {
    throw "no arguments provided"
}

for arg in args {
    let data = read(open(arg));
    print(data);
}