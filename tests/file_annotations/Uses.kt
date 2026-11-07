@file:RuntimeMark(TEXT, NUMBER)
@file:filemarks.Binary(true)
@file:SourceMark("gone")

package fileuses

import filemarks.Visible as RuntimeMark
import filemarks.Source as SourceMark

private const val TEXT: String = "hel" + "lo"
private const val NUMBER: Int = 3 + 4
fun answer(): Int = 42
val label: String = "value"
class Neighbor
