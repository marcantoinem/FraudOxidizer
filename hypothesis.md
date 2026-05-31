# Hypothesis Log

This file tracks the fraud hypotheses explored for this dataset and what we kept in the detector.

## Data Context

- Dataset: `transactions.csv`
- Scope: 1000 transactions, 50 cards
- Goal: high fraud recall without overwhelming reviewers with false positives

## Confirmed Hypotheses (from notebook + implementation)

## 1) Card Testing Burst

Hypothesis:
Stolen cards are often tested through many small online charges in a short period.

Rule kept:

- Same card
- Channel `online`
- At least 5 transactions
- Within 30 minutes
- Low median amount (micro-charges)

Why we kept it:

- Strong, interpretable signal
- Clusters naturally in time
- Easy for reviewer to validate from timeline context

## 2) Cashout / Resell Categories

Hypothesis:
Fraudsters cash out via high-value gift cards or electronics.

Rule kept:

- Category in `{gift_card, electronics}`
- Amount compared to card/category baselines
- Strong positive deviation (z-score style rule)

Why we kept it:

- Captures monetizable goods pattern
- Works best when relative to card behavior (not global fixed amount only)

## 3) Merchant Ring (Cross-Card)

Hypothesis:
Compromised or collusive merchants produce repeated high outliers across many cards.

Rule kept:

- Transaction amount significantly above merchant median
- Minimum outlier count at merchant level
- Minimum distinct cards impacted

Why we kept it:

- Invisible from single-card view
- High value cross-card aggregation signal

## 4) Impossible Travel / Foreign Country Mismatch

Hypothesis:
Physical transactions outside the cardholder's home country are suspicious.

Rule kept:

- In-person/ATM activity outside expected home country context
- Travel-aware logic to reduce false positives:
  - sustained foreign activity looks like travel
  - short isolated foreign burst is riskier

Why we kept it:

- High explanatory power for reviewer
- Improved precision after adding travel exception

## 5) Fraudulent Identity Link (Device/IP Reuse)

Hypothesis:
If a transaction shares device/IP with known suspicious activity, it is more likely fraud.

Rule kept:

- Match on device ID and/or IP with other suspicious transactions
- Confidence tiers:
  - stronger if linked source is human-confirmed fraud
  - weaker if linked source is only threshold-likely

Why we kept it:

- Provides feedback loop from reviewer decisions
- Adds cross-transaction propagation signal not limited to one card

## Scoring Hypothesis

Hypothesis:
Fraud risk should combine independent signals probabilistically, not by plain sum.

Rule kept:

- Noisy-OR combination:
  - `score = 1 - product(1 - weight_i)`
- Default flag threshold: `score >= 0.50`

Why we kept it:

- One weak signal alone rarely triggers a flag
- Multiple moderate signals combine into high-confidence alerts

## What We Explicitly Avoided

- Pure global static thresholds without per-card context
- One-signal hard blocking without reviewer confirmation path
- Country mismatch logic that ignores legitimate travel patterns

## Open Questions

- Should category deviation and card deviation thresholds adapt online based on reviewer outcomes?
- Should identity-link propagation include time decay (recent links weighted more)?
- Can we calibrate factor weights against a labeled validation split for better F1 stability?
