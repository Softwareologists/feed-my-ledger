# Scripting Examples

FeedMyLedger integrates the [Rhai](https://rhai.rs) scripting language. Use the
`run-script` command to execute a script against the current ledger. The script
receives an array named `records` where each entry is a map containing the
ledger fields.

Example script that totals cash expenses:

```rhai
let total = 0.0;
for r in records {
    if r.debit == "cash" {
        total += r.amount;
    }
}
print(total);
```

Run it with:

```bash
$ cargo run --bin ledger -- run-script --file report.rhai
```
