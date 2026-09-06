process.stdin.resume();
process.stdin.on("end", () => process.stdout.write("not-json\n"));
