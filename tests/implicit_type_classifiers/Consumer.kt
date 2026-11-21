package classifierprobe

fun <T : Any> consume(value: Catalog.Entry<T>): String = Lookup.value(value)
fun consumeText(value: Catalog.Entry<String>): String = Lookup.value(value)
fun observer(value: Observer, mode: Catalog.Mode) { value.changed(mode) }
