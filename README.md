# FraudOxidizer

FraudOxidizer is a Rust workspace for the Valsoft Fraud Hunter challenge.
It combines:

- A model crate that parses `transactions.csv`, computes fraud factors, and assigns an explainable fraud score.
- An interactive egui reviewer app (desktop + web via trunk) for triage and CSV export.
- A notebook (`fraud_patterns.ipynb`) used to explore and validate the core fraud hypotheses.

## What This Project Ships

This repository addresses the challenge requirements:

1. Ingest `transactions.csv` (1000 rows expected)
2. Flag suspicious transactions with a score (`fraud_score`)
3. Explain each flag with human-readable reasons (`FraudFactor::reason()`)
4. Provide a reviewer workflow (approve / mark fraud, undo / redo, threshold tuning)
5. Include README and hypothesis log
6. Export an updated CSV with a `likely_fraud` column that respects human review decisions

## Detection Strategy

The detector uses per-transaction fraud factors, then combines them with noisy-OR:

`score = 1 - product(1 - weight_i)`

Default fraud threshold: `0.50`.

### Main Fraud Signals

- Card Testing Burst
  - Rapid, small online transactions on the same card in a short window.
- Cashout-Like Category Risk
  - Gift card / electronics categories are treated as higher-risk contexts.
- Category Price Deviation
  - Amount is measured against category baseline using standard deviation bands (1z/2z/3z visualized in review charts).
- Card Amount Deviation
  - Amount deviates strongly from the card's own spending profile.
- Merchant Ring (cross-card)
  - Merchant with repeated high outliers across multiple distinct cards.
- Foreign Country Trip
  - Physical transactions in non-home country, with travel-aware handling to reduce false positives.
- Fraudulent Identity Link (cross-card)
  - Device/IP re-use from known suspicious transactions.
  - Confidence is stronger when linked transactions are human-confirmed fraud, weaker when only threshold-likely.

## Reviewer Experience

The UI is a 3-step flow:

1. Import CSV
2. Review flagged transactions
3. Overview + export reviewed CSV

Reviewer features:

- Threshold slider for human review queue
- One-by-one review carousel with contextual plots
- Keyboard shortcuts for navigation and decisions
- Bulk actions for card-testing burst series
- Undo/redo with activity log
- Combined-signal indicators in the UI
- Export that uses human decision when present (`TruePositive` / `FalsePositive`) and otherwise falls back to score threshold

## Run

Prerequisites:

- Rust toolchain (workspace uses edition 2024)
- For web UI: `trunk`

### Option A: Run the reviewer UI (native desktop)

From repo root:

```bash
cargo run -p fraud_oxidizer
```

### Option B: Run the reviewer UI (web)

From `crates/ui`:

```bash
trunk serve
```

Then open the local URL printed by trunk.

### Option C: Run model-only detection in terminal

From repo root:

```bash
cargo run -p model
```

This prints likely fraud transactions and their reason weights.

## Quality Checks

From repo root:

```bash
cargo fmt --all
cargo build --workspace
cargo clippy --workspace --all-targets
```

CI-like script:

```bash
./crates/ui/check.sh
```

## Notebook

`fraud_patterns.ipynb` documents and visualizes the four core challenge patterns:

1. Card Testing
2. Cashout
3. Merchant Ring
4. Impossible Travel

It also demonstrates noisy-OR combined scoring aligned with the Rust model.

## Output CSV

Exported CSV appends a `likely_fraud` column.

- `TruePositive` review forces `likely_fraud=true`
- `FalsePositive` review forces `likely_fraud=false`
- Unreviewed rows use model threshold decision

## What We Would Do With Another Week

- Add quantitative evaluation against labeled ground truth (precision / recall / F1 dashboard)
- Add confidence calibration and threshold tuning by explicit false-positive cost
- Improve factor-level suppression learning after reviewer feedback (beyond identity-link updates)
- Add richer audit trail export (who changed what, when, and why)
- Expand automated tests for every fraud pass and reviewer side effects
