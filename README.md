# verandah-image

Image composition and rendering helpers shared by
[verandah](https://github.com/sw1nn/verandah) and its widget plugins.

- `badge` — compose a badge (a mark on a filled disc) onto an icon
- `colors` — CSS named and hex colour parsing
- `font` — system font lookup via fontconfig, cached
- `image` — effects and format conversions
- `text` — text measurement and rendering

Nothing here crosses the plugin ABI, so verandah and a plugin may each link their
own version without conflict.

## License

MIT
