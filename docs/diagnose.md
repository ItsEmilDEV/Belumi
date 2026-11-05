# Diagnose report

`belumi-launcher --diagnose [--offline] [--server HOST[:PORT]] [--out FOLDER]` (or Game log -> Run diagnose in the
launcher) runs without any input and collects what is needed to find a problem on another machine.

What it does, in order:

1. Notes the system (operating system, launcher version, language) and runs the first-start checks.
2. For the shaders on and for the shaders off, in a scratch game folder (your saves and settings are not touched):
   - the game's standard benchmark (village, forest, ocean, mountain): average fps, 1 % lows, chunk load times, GPU
     memory, and the hardware the game itself detects (CPU, GPU, driver, backend, display);
   - a fresh world with four screenshots turning around, and the start times (resources loaded, first frame).
3. With a signed-in account and a server (`--server`, or a line in `diagnose-server.txt` beside the program), a short
   visit: does the join work, does the voice chat connect, plus screenshots.
4. Writes `summary.txt` (read this first), `summary.json`, the logs, benchmark reports, crash reports and screenshots
   into the report folder (default: `belumi-report` in the data folder), removes the home folder, the user name and
   anything shaped like a token from the text files, and packs it all into `belumi-report.zip`.

Nothing is sent anywhere: the folder is for the player to hand over.

The game's own side is `--diagnose-shots DIR` (screenshots, then quit), which the launcher passes.
