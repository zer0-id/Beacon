# Beacon

Beacon is an open-source, offline-first file transfer and sync tool between Android and Linux (and Android to Android), implemented with a Rust core and a Kotlin Android shell. It finds nearby devices over Bluetooth Low Energy, sets up a direct Wi-Fi link without any shared network, and then sends only the files that are missing on the other side, with rsync-style features such as diffing, resume and verification. It protects you from rogue peers by authenticating every device before any file inventory is exposed.

***

## Scope

- Nearby device discovery over BLE, with no central server and no internet requirement.
- Transfers that work with no shared network: the Android device hosts a temporary hotspot and the other device joins it as a plain Wi-Fi client.
- Direct device-to-device data transfer over TCP, encrypted and authenticated with the Noise protocol.
- Manifest-based diffing: compare what exists on each side and transfer only what is missing or changed.
- Recursive folder transfer with a preserved directory structure.
- Resumable transfers that survive dropped links, with chunk-level verification.
- A Linux CLI first, then a GUI.
- An Android app (Kotlin) for BLE, hotspot control, file access and UI.
- Security against malicious or rogue peers, malformed input and path-traversal attacks.
- A custom sync protocol.
- A probing mechanism to probe capabilities (hotspot support, Wi-Fi Direct etc.) and cache them for future use with an expiry.

***

## Operating Environment

The application will be built with a focus on Android and Linux systems.

- **Linux:** a Wi-Fi card that can connect as a client to a phone hotspot, and a working BlueZ stack for BLE.
- **Android:** a recent Android version with BLE, `LocalOnlyHotspot` (API 26+) or Wi-Fi Direct group support, and the required runtime permissions (`NEARBY_WIFI_DEVICES` on Android 13+).
- **Single Wi-Fi radio caveat:** the Linux machine may lose its internet connection during a transfer. The tool must reconnect to the previous network afterwards.

***

## Functional Requirements

- **Device Discovery:** Devices must discover each other over BLE using rotating anonymous identifiers and no device names, and not through a central discovery server.
- **Pairing:** First contact requires an out-of-band check (QR code or PIN) so that a man-in-the-middle cannot silently join. Paired devices store each other's static public keys for later sessions.
- **Link Setup:** The Android device starts a temporary hotspot with a random SSID and password, sends the credentials over the authenticated BLE channel, and the peer joins it. The hotspot is torn down when the session ends.
- **Peer Connections:** Once the link is up, the devices open a TCP connection and run a Noise handshake before any application data is exchanged.
- **Manifest Generation:** Each side builds a manifest per sync root, containing entry kind, relative path, size, mtime and a BLAKE3 content hash (plus chunk hashes for files). Manifests are streamed in bounded batches.
- **Diffing:** The receiver compares manifests and requests only missing or changed files or chunks. A dry-run mode shows what would be transferred without sending anything.
- **Recursive Folders:** Directories are explicit manifest entries so empty folders are preserved. Paths are relative, `/`-separated and Unicode-normalized.
- **File Chunking:** Files are divided into chunks (size TBD) to make resume, verification and progress tracking simple.
- **Chunk Verification:** Every chunk must be verified against its hash on arrival, and a file is marked complete only after all of its chunks are verified.
- **Resume:** After a disconnect, the session re-authenticates and continues from the last verified chunk using a persisted chunk bitmap.
- **Move Detection:** If a file's content hash already exists under a different path, the receiver renames it locally instead of re-downloading it.
- **User Interface:** A CLI on Linux first then a simple GUI later. The Android app provides device list, pairing, progress and diff results.
- **Peer Disconnects:** Disconnection from the peer, a sleeping phone or a dropped hotspot must not cause crashes or corruption. Partial files use temporary names and are renamed into place only after verification.

***

## Non-Functional Requirements

- The protocol framing and metadata overhead should stay under 1% of the payload size for files above a minimum size (to be validated by benchmarks).
- A 1 GB folder transfer must complete and resume after a forced disconnect without re-sending verified chunks.
- A second run on an unchanged folder must transfer zero file data.
- Hash caching keyed on (path, size, mtime) must keep rescans fast on low-spec devices.
- Memory use must stay bounded regardless of manifest size, through streaming and hard limits.
- Battery impact on Android must be reasonable, with the foreground service and hotspot active only during a session.

***

## QoL

- **Verification Pass:** A re-hash after transfer confirms both sides match.
- **Multiple Concurrent Shares:** The system should handle more than one sync root or file in the same session.
- **Daemon and IPC:** The daemon owns all transfer state. Frontends (CLI, TUI, GUI) are thin clients over a Unix socket or D-Bus.
- **Local IPC Safety:** The daemon socket lives in `$XDG_RUNTIME_DIR` with `0700` permissions.

***

## Security Requirements

- **Authenticate Before Exposure:** No manifest, file name or file data is sent before the Noise handshake and pairing are complete.
- **Transport Security:** All TCP traffic is encrypted and authenticated with Noise (`snow`), with the exact protocol string and prologue (including the protocol version) pinned in the spec. Hotspot credentials are never sent in plaintext over BLE.
- **First-Contact MITM Protection:** A short code derived from the handshake hash, or a QR code with the peer's key fingerprint, must be confirmed out of band on first pairing.
- **BLE Privacy:** Advertisements use rotating identifiers and expose no device name or user information.
- **Path Safety:** The receiver rejects paths containing `..`, absolute paths, empty components, NUL bytes and backslashes, and canonicalizes the final path to confirm it stays inside the destination root.
- **Symlinks:** Symlinks are not followed by default, and nothing is ever written through a symlink that points outside the root.
- **Resource Limits:** Maximum frame size, path length, directory depth, entry count, total size and per-state timeouts are enforced to resist memory-exhaustion attacks.
- **Strict State Machine:** Out-of-order or unexpected messages close the session. Any (state, message) pair not in the transition table goes to `Closed`.
- **Safe Permissions:** The sender's mode bits (setuid, world-writable) are not applied blindly.
- **Session Secrets:** The hotspot password and the pairing secret are one-time and rotated every session.
- **Dependency Hygiene:** `cargo audit` and `cargo deny` run in CI, and `unsafe` code is kept to a minimum and concentrated at the FFI boundary.
- **Honest Labeling:** The project is labeled "unaudited, experimental" until an independent review exists.

***

## Protocol Overview

- **Layers:** BLE for discovery and pairing, TCP over the Android-hosted hotspot for bulk data, Noise on top, and an application layer for manifests, chunks and resume.
- **Noise Patterns:** `XX` (plus out-of-band confirmation) on first contact, then `IK` for already-paired devices.
- **Hashes:** BLAKE3 for file and chunk integrity. Optional FastCDC chunking (rolling gear hash) for partially changed files, as a later tier. An rsync-style rolling checksum is only added if profiling shows it is needed.
- **Messages:** `HELLO`, `MANIFEST_BEGIN`, `MANIFEST_SCAFFOLD_BATCH` , `MANIFEST_BATCH`, `MANIFEST_END`, `WANT`, `CHUNK`, `ACK`, `DONE`, `ERROR`.
- **Session States:** `Idle`, `Pairing`, `Authenticated`, `Diffing`, `Transferring`, `Resuming`, `Done`, `Closed`.
- **Reliability:** TCP handles packet ordering and retransmission. The application layer tracks files and chunks and handles resume, not individual packets.
- **Versioning:** `HELLO` carries a protocol version and a session identifier so that replay and downgrade can be detected.
- **Formal Description:** The session is a Mealy-style finite state machine extended with data variables (the chunk bitmap). `Done` is the accepting state and `Closed` is the dead state.

***

## Architecture

- **`core` (Rust library):** protocol, Noise sessions, manifest, diff, chunker, transfer engine and hash cache (`rusqlite`).
- **`linux` (Rust):** BLE via `bluer`, Wi-Fi join via `zbus` and NetworkManager, the daemon and the CLI (`clap`).
- **`android` (Kotlin):** BLE, `LocalOnlyHotspot` or Wi-Fi Direct, permissions, foreground service, file access (SAF and MediaStore) and UI (Jetpack Compose). It calls `core` through UniFFI and passes file descriptors to Rust.

***

- **Transport Trait:** A `Transport` and `Handshake` abstraction keeps `core` testable.
- **Rule of Thumb:** Kotlin forwards raw events and file handles. Rust makes every trust decision.

***

## Testing and Verification

- **Unit Tests:** pure state machine tests that feed messages and assert states, including every invalid (state, message) pair reaching `Closed`.
- **Hostile Input Tests:** path validator tests with `../x`, absolute paths, symlink loops and Unicode lookalikes.
- **Fuzzing:** `cargo-fuzz` on the frame parser, manifest parser and state machine.
- **Resume Tests:** a localhost harness that kills the connection mid-transfer and checks that only unverified chunks are re-sent.
- **Formal Modeling:** model the handshake in Tamarin or ProVerif to check properties such as "no data accepted before authentication" and "no downgrade".
- **Hardware Matrix:** test on several Wi-Fi chipsets and several Android vendors, since BLE and hotspot behavior varies.
- **Rogue Peer Testing:** replay captured frames, spoof BLE advertisements and send malformed manifests in a lab environment.
- **External Review:** publish the spec for public review, then pursue a funded audit of the handshake, pairing and parser once the protocol is stable.
