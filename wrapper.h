// workaround some clang issue
#define __float128 long double
#define _HELENOS_SOURCE

#include <ui/ui.h>
#include <ui/wdecor.h>
#include <ui/window.h>
#include <ui/image.h>
#include <io/pixelmap.h>

// TODO: this is nice and works, but there's no inlining/optimization across
// Rust-C boundary. We should probably inline this, and also the pixelmap_* functions,
// directly into Rust
static inline pixel_t rgba_to_pix(uint8_t r, uint8_t g, uint8_t b, uint8_t a) {
    return PIXEL(a,r,g,b);
}
