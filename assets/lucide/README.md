# System action icons

SVG sources are from [Lucide 0.468.0](https://github.com/lucide-icons/lucide/tree/0.468.0/icons).
The picker embeds antialiased 20 × 20 RGBA assets. To regenerate one:

```sh
sed 's/currentColor/#e5e5e5/' power.svg |
    rsvg-convert -w 20 -h 20 |
    ffmpeg -v error -y -i pipe:0 -frames:v 1 -pix_fmt rgba -f rawvideo power.rgba
```
