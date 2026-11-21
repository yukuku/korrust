package consumers
import methodgenerics.*
fun result(): String {
    val generic = Generic<String>("seed")
    val named = Named("OK")
    val same: Named = generic.bounded(named)
    val second: String = generic.second(3, "!")
    val chosen: String = generic.choose(named)
    val first: String = generic.first(listOf("first"))
    val transformed: String = generic.transform(named) { value -> value.text() }
    val objectValue: Named = Helpers.bounded(named)
    return same.text() + second + chosen + first + transformed + objectValue.text()
}
fun interfaceResult(api: GenericApi<String>, value: Named): String {
    val resolved: Named = api.identity(value)
    val chosen: String = api.choose(3)
    return resolved.text() + chosen
}
