import factorylib.Value
import factorylib.Value as Renamed
import factorylib.Slot
import factorylib.Box
fun imported(): Value = Value(7)
fun alias(): Renamed = Renamed(8)
fun inferred(): Slot<String> = Slot("generic")
fun explicit(): Slot<Int> = Slot<Int>(9)
fun constructor(): Box = Box(10)
fun factory(): Box = Box("text")
