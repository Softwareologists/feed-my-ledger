use crate::core::Ledger;
use rhai::{Array, Dynamic, Engine, Map, Scope};

fn record_map(record: &crate::core::Record) -> Map {
    let mut map = Map::new();
    map.insert("id".into(), record.id.to_string().into());
    map.insert("description".into(), record.description.clone().into());
    map.insert("debit".into(), record.debit_account.to_string().into());
    map.insert("credit".into(), record.credit_account.to_string().into());
    map.insert("amount".into(), record.amount.into());
    map.insert("currency".into(), record.currency.clone().into());
    map.insert("cleared".into(), record.cleared.into());
    map
}

fn ledger_array(ledger: &Ledger) -> Array {
    ledger.records().map(record_map).map(Into::into).collect()
}

/// Execute a Rhai script against the provided `Ledger`.
pub fn run_script(script: &str, ledger: &Ledger) -> Result<Dynamic, Box<dyn std::error::Error>> {
    let mut scope = Scope::new();
    scope.push_constant("records", ledger_array(ledger));
    let engine = Engine::new();
    engine
        .eval_with_scope::<Dynamic>(&mut scope, script)
        .map_err(|e| e.into())
}
