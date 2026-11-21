package classifierprobe

fun <T> consume(value: Catalog.Entry<T>): String = Lookup.value(value)
fun consumeNullable(value: Catalog.Entry<String?>): String = Lookup.value(value)
fun observer(value: Observer, mode: Catalog.Mode) { value.changed(mode) }
