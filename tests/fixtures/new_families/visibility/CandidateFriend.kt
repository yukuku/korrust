package visibilityfriend
import visibilityprobe.*
fun touch(value: InternalDefault, item: Int): Int { value.value = item; return value.value }
fun partial(value: PublicInternalSetter, item: Int): Int { value.value = item; return value.value }
fun objectValue(item: Int): Int { State.hidden = item; return State.hidden }
fun classCompanion(item: Int): Int { Host.hidden = item; return Host.hidden }
fun interfaceCompanion(item: Int): Int { Contract.hidden = item; return Contract.hidden }
fun topValue(): Int = top
