# Security Policy

## Ethics & intended use

Akumo is a security-testing framework for **cloud infrastructure you are authorized to test** —
owned accounts, contracted engagements, and lab/CTF environments. Authorization gating, scope
enforcement, and audit are first-class, enforced requirements (`spec/requirements.md` §3, NFR-COMP).

Akumo deliberately does **not** ship capabilities whose only purpose is to evade an authorized
owner's defenses for malicious use. Defense-evasion behaviors are supported only as **labeled,
authorized, reversible** test actions (NFR-COMP4). Irreversible/destructive actions are opt-in,
gated, and prefer a simulated variant (FR-G6).

## Reporting a vulnerability

If you find a vulnerability **in Akumo itself** (e.g., a way it could leak credentials/loot, act
outside declared scope, or fail to revert), please report it privately:

- Open a GitHub *security advisory* on this repository (preferred), or
- Email the maintainers listed in the repository metadata.

Please include a minimal reproduction and the affected version/commit. Do **not** file a public
issue for an unpatched vulnerability. We aim to acknowledge within a few business days.

Do not include working exploits against third-party cloud providers, real credentials, or customer
data in a report — describe the class of problem instead.

## Handling of secrets

Akumo stores credentials and loot **encrypted at rest** and **redacts secrets by default** in logs
and reports (NFR-SEC1/2). It never transmits target data, credentials, or loot to any third party
without explicit operator opt-in (NFR-SEC3). Never paste real secrets into issues or PRs.
