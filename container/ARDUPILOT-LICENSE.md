# The ArduPilot image's licence

The image `aethelgard-ardupilot`, built from `container/ardupilot.Dockerfile`, carries:

- **ArduPilot's complete source** at tag `Copter-4.7.2`, commit `fb773567896014da007bca58358704d4a0d8516f`, with every submodule at the commit that tag records. It is under `/opt/ardupilot`, with its `.git` directory. ArduPilot is licensed under the **GNU General Public License version 3** (`COPYING.txt` in that tree), and its source files say "or (at your option) any later version", so the image is labelled `GPL-3.0-or-later`. The image is therefore a distribution of GPL source in itself.
- **The GNU Arm Embedded Toolchain** `gcc-arm-none-eabi-10-2020-q4-major`, Arm's binary release, as ArduPilot's firmware server serves it (sha256 in the Dockerfile). It carries its own licence files under `/opt/gcc-arm-none-eabi-10-2020-q4-major/share/doc/`. Its corresponding source is Arm's source tarball of the same release, mirrored byte for byte next to this image at the digest in `container/GCC-SOURCE`, as `4400a` D-2 ruled (`AET-WO-4403`). An anonymous client can fetch it.
- **Python packages** installed by hash from PyPI into `/opt/venv`, each under its own licence, and **Debian 13 packages**, each under its own licence in `/usr/share/doc/<package>/copyright`.

The written offer of every GPL or LGPL component's source is `/SOURCES.md` in the image, which is `container/ARDUPILOT-SOURCES.md`.

**Nothing GPL enters a crate, the tools image or the engine image.** No crate in this repository vendors, links or includes ArduPilot code or binaries. The pinout check (`crates/aethelgard-pincheck`) reads a firmware ELF the way `readelf` does; it holds no ArduPilot code. No ArduPilot binary is committed: the ELFs it judges exist only in this image's build directory and in CI artifacts.

The `hwdef.dat` files under `crates/aethelgard-pincheck/fixtures/` are this project's own, written in `AET-WO-4400`, and are licensed with that crate (`MIT OR Apache-2.0`).
