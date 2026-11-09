# Contributing

Thanks for helping with OpenDJDecks.

## Build and test

```sh
CARGO_INCREMENTAL=0 cargo test --workspace
cargo run -p od-parity --bin parity -- check --strict
packaging/ci/check-legal.sh
```

The website lives in `website/`: `npm ci`, then `npm run build`, `npm test`, `npm run check-links`.

German interface strings are in `crates/od-ui/lang/de/LC_MESSAGES/od-ui.po`. Guides exist in
`docs/guide/en` and `docs/guide/de` and stay in step.

## Independence

These rules protect the project and everyone who works on it. `packaging/ci/check-legal.sh` checks
what a script can check. The rest is on us.

**Sources, in this order**

1. Public documentation and open-source projects, under their licences.
2. Observing software in normal use and capturing traffic between your own computer and your own
   hardware.

Do not disassemble or decompile third-party programs or firmware, and do not paste such output
into a tool, an issue or a commit. If something cannot be found out this way, leave it out and
open an issue.

**Write it down in your own words first.** Describe a format or protocol in plain language and
implement it from that description. Constants that a format or protocol requires are fine. Names,
structure and tables taken from someone else's program are not.

**Never commit**

- third-party binaries, firmware, drivers, or parts of them
- icons, fonts, skins, sounds, sample packs, demo tracks or screenshots from other DJ software
- manual text or manual illustrations
- exported libraries, captures containing serial numbers, account data or tokens
- a key for encrypted libraries, or code that fetches one
- anything that unlocks licences, subscriptions or hardware-bound features of other software

**Own look.** Standard DJ layouts are fine. Copying another product's colour scheme, icons or
screen composition is not. Do not work from screenshots of other DJ software.

**Names.** Other companies' product names appear only in running text to say what something works
with, never in the name of a file, crate, feature, button or package. Exceptions are paths and tags
that a file format requires.

**Other people's systems.** Features that touch a network talk only to devices the user controls.
Report security problems in other products to their makers, not here.

Add new sources to [NOTICE.md](NOTICE.md).
