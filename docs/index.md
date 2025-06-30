---
title: Rusty Ledger
---
<link href="https://fonts.googleapis.com/css2?family=Inter:wght@500;700&family=Source+Code+Pro&display=swap" rel="stylesheet">

<style>
body {
  font-family: 'Inter', sans-serif;
  background-color: #F0F8FF;
  color: #2F4F4F;
  margin: 0;
  padding: 2rem;
}

h1, h2 {
  font-weight: 700;
  letter-spacing: 0.05em;
  text-transform: uppercase;
}

h2 {
  margin-top: 2rem;
}

button.primary {
  background-color: #B7410E;
  color: white;
  border: none;
  border-radius: 8px;
  padding: 0.5rem 1rem;
  font-size: 1rem;
}

button.secondary {
  background-color: transparent;
  color: #2F4F4F;
  border: 2px solid #2F4F4F;
  border-radius: 8px;
  padding: 0.5rem 1rem;
  font-size: 1rem;
}

code, pre {
  font-family: 'Source Code Pro', monospace;
  background-color: #F0F8FF;
  border-left: 4px solid #A0A8B7;
  padding: 1rem;
  overflow-x: auto;
}

.features li {
  margin-bottom: 0.5rem;
}
</style>
<img src="https://private-user-images.githubusercontent.com/1645545/459125430-6c630732-3bc5-43ac-bcb7-ade199cefcc2.png?jwt=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJnaXRodWIuY29tIiwiYXVkIjoicmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbSIsImtleSI6ImtleTUiLCJleHAiOjE3NTEyNDM2MjUsIm5iZiI6MTc1MTI0MzMyNSwicGF0aCI6Ii8xNjQ1NTQ1LzQ1OTEyNTQzMC02YzYzMDczMi0zYmM1LTQzYWMtYmNiNy1hZGUxOTljZWZjYzIucG5nP1gtQW16LUFsZ29yaXRobT1BV1M0LUhNQUMtU0hBMjU2JlgtQW16LUNyZWRlbnRpYWw9QUtJQVZDT0RZTFNBNTNQUUs0WkElMkYyMDI1MDYzMCUyRnVzLWVhc3QtMSUyRnMzJTJGYXdzNF9yZXF1ZXN0JlgtQW16LURhdGU9MjAyNTA2MzBUMDAyODQ1WiZYLUFtei1FeHBpcmVzPTMwMCZYLUFtei1TaWduYXR1cmU9MWM0NmZiZmUyYzdkN2Y0ZTFhZjMwODQzNmFhNTA0YzZhZGQ2ZjNhZTNlNmNmZTE0YjBjNjYyMmJkYzJjMzdmOSZYLUFtei1TaWduZWRIZWFkZXJzPWhvc3QifQ.s7f2fGHSYY8fF8NL_DDY8J9UiRaFebx_X-ZK4kV2zCM" alt="Rusty Ledger Logo" style="max-width: 150px;"/>

<h1>Rusty Ledger</h1>
<p style="font-weight:500;">Immutable ledgers, evolved.</p>

<h2>Features</h2>
<ul class="features">
<li>Immutable data entries.</li>
<li>Append-only adjustments.</li>
<li>Cloud service integration.</li>
<li>User authentication via OAuth2.</li>
<li>Data sharing with granular permissions.</li>
<li>Resilient API calls with retries.</li>
</ul>

<h2>Usage</h2>

<pre><code>use rusty_ledger::core::{Ledger, Record};

let mut ledger = Ledger::default();
let record = Record::new(
    "Sample transaction".into(),
    "cash".into(),
    "revenue".into(),
    100.0,
    "USD".into(),
    None,
    None,
    vec!["example".into()],
).unwrap();
ledger.append(record);
</code></pre>

<p>
  <a href="../README.md"><button class="primary">Get Started</button></a>
  <a href="README.md"><button class="secondary">Documentation</button></a>
</p>

