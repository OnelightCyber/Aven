// Linux builds do not compile yet, so stop before Cargo with a clear note.
if (process.platform === "linux") {
  console.error(
    "Aven isn't available on Linux yet. We're working on it; follow https://aven.invalid for news.",
  );
  process.exit(1);
}
