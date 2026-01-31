# Onyx
<img src="./src/onyx.png" width="128" align="center">

**Onyx** is a minimalist runtime designed to manage isolated rootfs environments with zero daemon overhead, built for systems where every megabyte of RAM counts.

---

## Core Features

v0.1.1:

* **Portability:** Onyx is a ~1MB binary that handles everything, like updates, box management, and performance profiles.
* **Unprivileged Execution:** Native support for `proot` to provide guest-side root simulation without host-side `sudo`.
* **Performance Profiles:** Hard limit your box's resource usage easily and automatically using either the default profiles, or your own!
* **Organized Files:** Everything is kept in /home/onyx!
* **Optimized:** Compiled binary size optimized for 32mb (and less!) ramdisk environments.
* **Host-agnostic:** Runs on any Linux kernel that supports basic namespaces or `proot`! (latest officially supported: kernel 4.14)
* **Mount Safe:** Uses unshared and guarded private mounts for Onyx box execution.

---

## Usage

```bash
# install onyx
...

# create a new box environment (can be debootstrap or anything that generates a rootfs)
onyx box create my-box ./linux-rootfs FALSE

# launch the environment
onyx box open my-box
```

---

## Coming Soon

* `onyx lux`: An API for users to create plugins (daemons, autostarts, and much more), extensions (subcommands of modules), and user modules (fully fledged custom content)!
* **Layers:** Use multiple layers to keep your main rootfs safe!
