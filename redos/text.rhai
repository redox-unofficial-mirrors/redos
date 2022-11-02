for arg in args {
    print(`arg: ${arg}`);
}

print("Testing error");
try {
    let fd = open("redos/notfound.txt");
} catch (err) {
    print(`error: ${err}`);
}

print("Testing read");
let fd = open("redos/text.txt");
let data = read(fd);
print(`data: ${data}`);
