# Enshrouded Server Manager

A native Windows app, built with Rust and [GPUI](https://www.gpui.rs), for configuring and managing an
Enshrouded dedicated server through its `enshrouded_server.json`.

## Run

```
cargo run --release
```

By default it edits
`C:/Program Files (x86)/Steam/steamapps/common/EnshroudedServer/enshrouded_server.json`.
Pass another path as the first argument to edit a different file:

```
cargo run --release -- "D:/Servers/Enshrouded/enshrouded_server.json"
```

Building needs the Windows SDK (GPUI compiles its shaders with `fxc.exe`).

## What it does

- **Overview**: server status, key numbers, a health check and shortcuts to the server, save and log folders.
  Start and stop the server process from here.
- **Server**: name, slots, IP and port, directories, text and voice chat, discovery tags.
- **Gameplay**: difficulty presets plus every individual game setting, with the ranges from the server readme.
  The individual values only apply with the `Custom` preset, and the UI says so.
- **Players**: user groups with masked passwords, a password generator, the five permissions and reserved slots.
- **Bans**: view and lift banned accounts.
- **Review**: every pending change against the file on disk, and a preview of the exact JSON to be written.
- **Backups**: a snapshot is taken before every save (newest 30 kept) and can be restored.

## Safety

- Saves are atomic (temporary file, then rename) and the previous file is backed up first.
- Unknown keys and key order in the JSON are preserved, so future server versions keep working.
- Saving is blocked while validation errors exist, for example duplicate group passwords or a bad port.
- Closing the window with unsaved edits asks first.
- If the file changes on disk while you have no edits, it reloads. If you do have edits, you are warned.
- Stopping the server asks it to close gracefully so it saves the world. A force stop is offered if it hangs.

Shortcut: `Ctrl+S` saves.
