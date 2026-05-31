# Hypothesis

## Wrong country hypothesis

For a given card, a sudden switch to another country over a short time window is likely fraud.

Travel is the main exception:

- If transactions in the foreign country form a sustained trip (multiple transactions or long enough duration), classify this as likely travel.
- If the foreign activity is brief and isolated, classify it as suspicious.

In short: short foreign bursts are risky, sustained foreign presence is more likely legitimate travel.
