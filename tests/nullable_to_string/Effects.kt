import stringify.Probe
fun nullable(value: Any?): String = value.toString()
fun literalNull(): String = null.toString()
fun nullableString(value: String?): String = value.toString()
fun number(value: Int?): String = value.toString()
fun bool(value: Boolean?): String = value.toString()
fun character(value: Char?): String = value.toString()
fun chars(value: CharArray?): String = value.toString()
fun references(value: Array<String>?): String = value.toString()
fun <T> generic(value: T): String = value.toString()
fun next(mode: Int): String { val value: Probe? = Probe.next(mode); return value.toString() }
fun safe(value: Probe?): String? = value?.toString()
fun bound(value: Any?): String { val render: () -> String = value::toString; return render() }
