package ten.external

import yuku.alkitab.base.dialog.VersesDialogNormalVerses
import yuku.alkitab.base.dialog.XrefDialogVerses

fun normal(): Any = VersesDialogNormalVerses(listOf("a"), listOf("1"))
fun xref(): Any = XrefDialogVerses("missing", listOf(null), listOf("1"))
