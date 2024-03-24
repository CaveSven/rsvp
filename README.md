# rsvp

A Rust binary to interoperate between `neomutt` and `khal`.

This program can be used to add or remove meeting invitations from your (neomutt) inbox to khal. When choosing the
participation status, neomutt will email the organizer informing them about your participation.

## Setup

Build the binary and copy it to some reasonable location:

`cargo build --release && cp target/release/rsvp ~/.local/bin/rsvp`

Configure neomutt to use `rsvp` for `text/calendar` mimetypes in its `mailcap` file:

```mailcap
...
text/calendar; ~/.local/bin/rsvp -- %s <path_to_neomutt_config>;
...
```

The idea behind providing an explicit muttrc file is to use a slimmed down version, e.g. without gpg config and / or
email signature in order to reduce boilerplate.

## Usage

If you receive an ics invitation in your inbox, open the message, display the different parts (`l`) navigate to
the `text/calendar` entry and open it. You will then be asked about your participation status.