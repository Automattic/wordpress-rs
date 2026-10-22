package rs.wordpress.api.android

import android.graphics.Color
import uniffi.wp_mobile.ReaderColor
import uniffi.wp_mobile.readerColorFromArgb

/**
 * The colour from an Android colour int (a `@ColorInt`), like `ContextCompat.getColor` and
 * `MaterialColors.getColor` give. Theme colours look different in light and dark mode, so read each
 * one from a context with the configuration you want.
 */
fun ReaderColor.Companion.fromColorInt(colorInt: Int): ReaderColor = readerColorFromArgb(colorInt)

/** The colour as an Android colour int (a `@ColorInt`). */
fun ReaderColor.toColorInt(): Int = argb()

/**
 * The colour in sRGB, from an Android [Color] in any colour space. A colour in a wider colour space,
 * like Display P3, is converted to sRGB, and what sRGB can't show is clamped.
 */
fun ReaderColor.Companion.fromColor(color: Color): ReaderColor = fromColorInt(color.toArgb())

/** The colour as an Android [Color], in sRGB. */
fun ReaderColor.toColor(): Color = Color.valueOf(toColorInt())
