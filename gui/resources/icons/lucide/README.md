# Lucide icon subset

These SVGs come from the official
[Lucide repository](https://github.com/lucide-icons/lucide/tree/main/icons).
Clean the Agent vendors only the symbols used by the interface and compiles
them into the executable.

The Rust icon layer may split a symbol into independently rendered SVG paths so
motion can target a meaningful part of the glyph instead of scaling the
complete icon. Keep the 24 × 24 view box, 2 px stroke, round caps, and round
joins when adding or adapting a symbol.

See [`../../licenses/lucide-LICENSE.txt`](../../licenses/lucide-LICENSE.txt) for the ISC and applicable Feather MIT terms.
