package simple.invalid

import yuku.alkitab.songs.SongTextZoomable

class ReadOnly() : SongTextZoomable {
    override val songTextZoomPercent: Int
        get() = 100
}
