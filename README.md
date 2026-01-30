# onyx
<img src="./src/onyx.png" width="128" align="center">

**onyx** is a minimalist runtime designed to manage isolated rootfs environments with zero daemon overhead, built for systems where every megabyte of ram counts.

---

## 🏗 core features

v0.1.0 focuses on the fundamental "muscle" of the system:

* **the supervisor:** a single rust binary that handles process isolation and lifecycle management.
* **unprivileged execution:** native support for `proot` to provide guest-side root simulation without host-side `sudo`.
* **static state-tree:** persistent tracking of active boxes located in `/home/onyx/sys/active`.
* **resource pinning:** initial support for pinning processes to specific cpu cores.



---

## 🦾 technical principles

1.  **no-daemon architecture:** onyx launches, supervises the child process, and exits when the box is shut down.
2.  **minimalism:** compiled binary size optimized for 16mb ramdisk environments.
3.  **host-agnostic:** runs on any linux kernel that supports basic namespaces or `proot`.

---

## 🛠 usage

```bash
# create a new box environment
onyx create my-box ./linux-rootfs

# launch the environment
onyx run my-box
```
