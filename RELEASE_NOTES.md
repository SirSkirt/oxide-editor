# Oxide Editor B1.3.2

B1.3.2 begins Oxide's native package-update system.

- Existing Oxide installations can update from dedicated signed ZIP packages instead of running the Windows installer for every update.
- Oxide downloads and cryptographically verifies the package before handing it to the Oxide Update Service.
- The Oxide Update Service backs up the current runtime files, replaces them, rolls back on installation failure, and restarts the editor.
- The update helper uses its own Oxide-styled interface.
- The normal NSIS installer remains available for first-time installation and repair.
