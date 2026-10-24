package outside

import simple.mutable.MutableSurface
import yuku.alkitab.songs.SongTextZoomable

fun invalid(surface: MutableSurface, zoom: SongTextZoomable) {
    zoom.songTextZoomPercent = null
    surface.label = null
    surface.bytes = null
    surface.chars = null
    surface.lines = arrayOf<String?>(null)
}
