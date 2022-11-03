let line = "";
for arg in args {
    if line.len() > 0 {
        line += " ";
    }
    line += arg;
}
print(line);