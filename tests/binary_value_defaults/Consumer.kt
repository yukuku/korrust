package binarydefaults.consumer
import binarydefaults.library.Key
import binarydefaults.library.Label
import binarydefaults.library.readerEvent
import binarydefaults.library.nullableMix
import binarydefaults.library.Reader
import binarydefaults.library.top
import binarydefaults.library.mix
import binarydefaults.library.reference
import binarydefaults.library.nullable
import binarydefaults.library.wide
import binarydefaults.library.reset
import binarydefaults.library.keyEvent
import binarydefaults.library.textEvent
fun topValue(): Long = top().raw + top(Key(31L), Label("given")).raw
fun memberValue(): Long = Reader().read().raw + Reader().read(label = "named").raw
fun extensionValue(): Long = Key(29L).mix().raw
fun dispatchValue(): Long = Key(37L).plus().raw
fun referenceValue(): String = reference().raw
fun nullableValue(): Boolean = nullable()
fun genericValue(): String = Reader().generic(item = "generic")
fun wideValue(): Long = wide(p32 = 99)
fun order(): String {
    reset()
    return Reader().ordered(text = textEvent("a"), key = keyEvent("b"))
}
fun defaultOrder(): String {
    reset()
    return readerEvent().ordered(text = textEvent("a"))
}

fun nullableExtension(): Long {
    val key: Key? = null
    return key.nullableMix()
}
